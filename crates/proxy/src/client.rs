//! OpenAI-compatible LLM client.
//!
//! Approaches build request bodies as raw JSON (exactly the kwargs upstream
//! passes to `client.chat.completions.create(**kwargs)`) and get back the
//! response normalised the way openai-python's `model_dump()` shapes it.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Map, Value};

use crate::py;

/// Error returned by a provider call, formatted like openai-python's exceptions.
#[derive(Debug, Clone)]
pub struct LlmError {
    pub status: Option<u16>,
    pub message: String,
    pub body: Option<Value>,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LlmError {}

/// Anything that can serve `chat.completions.create` and `models.list`.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// `client.chat.completions.create(**body)`.
    async fn create(&self, body: Value) -> std::result::Result<ChatCompletion, LlmError>;

    /// `client.models.list()` as `{"object": "list", "data": [...]}`.
    async fn list_models(&self) -> std::result::Result<Value, LlmError>;

    /// `client.models.retrieve(model)` (openai-python `Model`, extras kept).
    async fn retrieve_model(&self, model: &str) -> std::result::Result<Value, LlmError> {
        let _ = model;
        Err(LlmError {
            status: Some(404),
            message: "models.retrieve is not supported by this client".into(),
            body: None,
        })
    }

    /// Base URL of the endpoint (used by plugins that build sibling clients).
    fn base_url(&self) -> String;

    /// API key in use (used by plugins that build sibling clients).
    fn api_key(&self) -> String;

    /// A copy of an openai-python-style client with `max_retries` replaced;
    /// `None` for other client kinds (upstream passes those through as-is).
    fn with_retries(&self, max_retries: u32) -> Option<SharedClient> {
        let _ = max_retries;
        None
    }
}

pub type SharedClient = Arc<dyn LlmClient>;

/// Provider flavour, mirroring the branches of upstream `get_config()`.
#[derive(Clone, Debug, PartialEq)]
pub enum Flavor {
    OpenAi,
    Cerebras,
    Azure { api_version: String },
}

/// HTTP client for OpenAI-compatible endpoints with openai-python's retry policy.
#[derive(Clone)]
pub struct OpenAiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    flavor: Flavor,
    max_retries: u32,
}

impl OpenAiClient {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::with_flavor(api_key, base_url, Flavor::OpenAi, true, None)
    }

    pub fn with_flavor(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        flavor: Flavor,
        ssl_verify: bool,
        ssl_cert_path: Option<&str>,
    ) -> Self {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .connect_timeout(Duration::from_secs(5));
        if !ssl_verify {
            builder = builder.danger_accept_invalid_certs(true);
        } else if let Some(path) = ssl_cert_path.filter(|p| !p.is_empty()) {
            if let Ok(pem) = std::fs::read(path) {
                if let Ok(cert) = reqwest::Certificate::from_pem(&pem) {
                    builder = builder.add_root_certificate(cert);
                }
            }
        }
        let mut base_url: String = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self {
            http: builder.build().expect("reqwest client"),
            base_url,
            api_key: api_key.into(),
            flavor,
            max_retries: 2,
        }
    }

    /// openai-python's `max_retries` client option.
    pub fn with_max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    fn chat_url(&self, model: &str) -> String {
        match &self.flavor {
            Flavor::Azure { api_version } => format!(
                "{}/openai/deployments/{}/chat/completions?api-version={}",
                self.base_url, model, api_version
            ),
            Flavor::Cerebras => format!("{}/v1/chat/completions", self.base_url),
            Flavor::OpenAi => format!("{}/chat/completions", self.base_url),
        }
    }

    fn models_url(&self) -> String {
        match &self.flavor {
            Flavor::Azure { api_version } => {
                format!(
                    "{}/openai/models?api-version={}",
                    self.base_url, api_version
                )
            }
            Flavor::Cerebras => format!("{}/v1/models", self.base_url),
            Flavor::OpenAi => format!("{}/models", self.base_url),
        }
    }

    fn authorize(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.flavor {
            Flavor::Azure { .. } => req.header("api-key", &self.api_key),
            _ => req.bearer_auth(&self.api_key),
        }
    }

    async fn send_with_retries(
        &self,
        build: impl Fn() -> reqwest::RequestBuilder,
    ) -> std::result::Result<Value, LlmError> {
        let mut attempt = 0u32;
        loop {
            let result = build().send().await;
            match result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let headers = resp.headers().clone();
                    let text = resp.text().await.unwrap_or_default();
                    if (200..300).contains(&status) {
                        return serde_json::from_str(&text).map_err(|e| LlmError {
                            status: Some(status),
                            message: format!("Invalid JSON from provider: {}", e),
                            body: None,
                        });
                    }
                    let should_retry =
                        match headers.get("x-should-retry").and_then(|v| v.to_str().ok()) {
                            Some("true") => true,
                            Some("false") => false,
                            _ => matches!(status, 408 | 409 | 429) || status >= 500,
                        };
                    if should_retry && attempt < self.max_retries {
                        tokio::time::sleep(retry_delay(attempt, &headers)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(status_error(status, &text));
                }
                Err(e) => {
                    if attempt < self.max_retries {
                        tokio::time::sleep(retry_delay(
                            attempt,
                            &reqwest::header::HeaderMap::new(),
                        ))
                        .await;
                        attempt += 1;
                        continue;
                    }
                    let message = if e.is_timeout() {
                        "Request timed out.".to_string()
                    } else {
                        "Connection error.".to_string()
                    };
                    return Err(LlmError {
                        status: None,
                        message,
                        body: None,
                    });
                }
            }
        }
    }
}

/// openai-python `_calculate_retry_timeout`.
fn retry_delay(retries_taken: u32, headers: &reqwest::header::HeaderMap) -> Duration {
    if let Some(ms) = headers
        .get("retry-after-ms")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<f64>().ok())
    {
        let secs = ms / 1000.0;
        if (0.0..=60.0).contains(&secs) {
            return Duration::from_secs_f64(secs);
        }
    }
    if let Some(secs) = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<f64>().ok())
    {
        if (0.0..=60.0).contains(&secs) {
            return Duration::from_secs_f64(secs);
        }
    }
    let base = (0.5 * 2f64.powi(retries_taken as i32)).min(8.0);
    let jitter = 1.0 - 0.25 * py::random::PyRandom::from_entropy().random();
    Duration::from_secs_f64(base * jitter)
}

/// openai-python `_make_status_error_from_response` message: `Error code: N - <body>`.
pub fn status_error(status: u16, text: &str) -> LlmError {
    let trimmed = py::strings::strip(text);
    let body: Option<Value> = serde_json::from_str(trimmed).ok();
    let shown = match &body {
        Some(v) => py::json::repr(v),
        None => trimmed.to_string(),
    };
    LlmError {
        status: Some(status),
        message: format!("Error code: {} - {}", status, shown),
        body,
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    fn with_retries(&self, max_retries: u32) -> Option<SharedClient> {
        Some(Arc::new(self.clone().with_max_retries(max_retries)))
    }

    async fn create(&self, body: Value) -> std::result::Result<ChatCompletion, LlmError> {
        check_create_kwargs(&body)?;
        let body = merge_extra_body(body);
        let model = body
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_string();
        let url = self.chat_url(&model);
        let raw = self
            .send_with_retries(|| self.authorize(self.http.post(&url)).json(&body))
            .await?;
        Ok(ChatCompletion::from_raw(raw))
    }

    async fn list_models(&self) -> std::result::Result<Value, LlmError> {
        let url = self.models_url();
        let raw = self
            .send_with_retries(|| self.authorize(self.http.get(&url)))
            .await?;
        let data = raw
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        let schema = &schema()["Model"];
        let data: Vec<Value> = data.iter().map(|m| normalize(m, schema)).collect();
        Ok(json!({"object": "list", "data": data}))
    }

    async fn retrieve_model(&self, model: &str) -> std::result::Result<Value, LlmError> {
        let url = format!("{}/{}", self.models_url(), model);
        let raw = self
            .send_with_retries(|| self.authorize(self.http.get(&url)))
            .await?;
        Ok(normalize(&raw, &schema()["Model"]))
    }

    fn base_url(&self) -> String {
        self.base_url.clone()
    }

    fn api_key(&self) -> String {
        self.api_key.clone()
    }
}

/// True when `Completions.create` declares a keyword argument named `k`.
pub fn is_create_param(k: &str) -> bool {
    schema()["create_params"]
        .as_array()
        .map(|a| a.iter().any(|p| p.as_str() == Some(k)))
        .unwrap_or(false)
}

/// `Completions.create(**kwargs)` raises `TypeError` for keyword arguments it
/// does not declare; upstream surfaces that as the request's error string.
pub fn check_create_kwargs(body: &Value) -> std::result::Result<(), LlmError> {
    let allowed = schema()["create_params"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(obj) = body.as_object() {
        for k in obj.keys() {
            if !allowed.iter().any(|a| a.as_str() == Some(k.as_str())) {
                return Err(LlmError {
                    status: None,
                    message: format!(
                        "Completions.create() got an unexpected keyword argument '{}'",
                        k
                    ),
                    body: None,
                });
            }
        }
    }
    Ok(())
}

/// openai-python merges `extra_body` into the JSON body and drops the key itself.
pub fn merge_extra_body(mut body: Value) -> Value {
    if let Some(obj) = body.as_object_mut() {
        // Client-side options, never part of the JSON body.
        for k in ["extra_headers", "extra_query", "timeout"] {
            obj.shift_remove(k);
        }
        if let Some(Value::Object(extra)) = obj.shift_remove("extra_body") {
            for (k, v) in extra {
                obj.insert(k, v);
            }
        }
    }
    body
}

fn schema() -> &'static Value {
    static SCHEMA: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("openai_schema.json")).expect("openai schema")
    })
}

/// Recreate pydantic `model_dump()` of a `construct()`ed model: declared
/// fields first (null when missing), then any extra keys in original order.
pub fn normalize(value: &Value, schema: &Value) -> Value {
    if let Some(item_schema) = schema.get("list") {
        return match value {
            Value::Array(items) => {
                Value::Array(items.iter().map(|v| normalize(v, item_schema)).collect())
            }
            other => other.clone(),
        };
    }
    if let Some(Value::Object(variants)) = schema.get("union") {
        let chosen = value
            .get("type")
            .and_then(|t| t.as_str())
            .and_then(|t| variants.get(t))
            .or_else(|| variants.values().next());
        return match chosen {
            Some(s) => normalize(value, s),
            None => value.clone(),
        };
    }
    if let (Some(Value::Array(fields)), Value::Object(obj)) = (schema.get("fields"), value) {
        let mut out = Map::new();
        for field in fields {
            let name = field[0].as_str().unwrap_or_default();
            let sub = &field[1];
            let v = obj.get(name).cloned().unwrap_or(Value::Null);
            out.insert(
                name.to_string(),
                if v.is_null() { v } else { normalize(&v, sub) },
            );
        }
        for (k, v) in obj {
            if !out.contains_key(k) {
                out.insert(k.clone(), v.clone());
            }
        }
        return Value::Object(out);
    }
    value.clone()
}

/// A chat completion response, normalised like `response.model_dump()`,
/// plus the wire JSON (what `model_dump(exclude_unset=True)` yields when the
/// SDK echoes a response object back into a request).
#[derive(Clone, Debug)]
pub struct ChatCompletion(pub Value, pub Value);

impl ChatCompletion {
    pub fn from_raw(raw: Value) -> Self {
        ChatCompletion(normalize(&raw, &schema()["ChatCompletion"]), raw)
    }

    /// `response.choices[i].message` as sent on the wire.
    pub fn raw_message(&self, i: usize) -> Option<&Value> {
        self.1.get("choices")?.get(i)?.get("message")
    }

    pub fn choices(&self) -> &[Value] {
        self.0
            .get("choices")
            .and_then(|c| c.as_array())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// `response.choices[i].message.content` (None when null / missing).
    pub fn content(&self, i: usize) -> Option<String> {
        self.choices()
            .get(i)?
            .get("message")?
            .get("content")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// `response.choices[i].message.content`, raising like Python on a missing choice.
    pub fn content_at(&self, i: usize) -> Result<Option<String>> {
        if i >= self.choices().len() {
            anyhow::bail!("list index out of range");
        }
        Ok(self.content(i))
    }

    pub fn finish_reason(&self, i: usize) -> Option<String> {
        self.choices()
            .get(i)?
            .get("finish_reason")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// `response.choices[i].message` as JSON.
    pub fn message(&self, i: usize) -> Option<&Value> {
        self.choices().get(i)?.get("message")
    }

    /// `response.usage.completion_tokens`, failing like Python when usage is absent.
    pub fn completion_tokens(&self) -> Result<i64> {
        match self.0.get("usage") {
            Some(Value::Object(u)) => match u.get("completion_tokens") {
                Some(Value::Number(n)) => Ok(n.as_i64().unwrap_or(0)),
                _ => anyhow::bail!("unsupported operand type(s) for +=: 'int' and 'NoneType'"),
            },
            _ => anyhow::bail!("'NoneType' object has no attribute 'completion_tokens'"),
        }
    }

    /// `response is None or not response.choices or content is None or finish_reason == "length"`.
    pub fn is_unusable(&self) -> bool {
        self.choices().is_empty()
            || self.content(0).is_none()
            || self.finish_reason(0).as_deref() == Some("length")
    }

    /// `response.usage.reasoning_tokens` style accessors.
    pub fn usage(&self) -> Option<&Value> {
        self.0.get("usage").filter(|u| !u.is_null())
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_matches_model_dump() {
        let raw = json!({"id":"x","choices":[{"message":{"content":"hi"}}]});
        let c = ChatCompletion::from_raw(raw);
        let expected = json!({"id": "x", "choices": [{"finish_reason": null, "index": null, "logprobs": null, "message": {"content": "hi", "refusal": null, "role": null, "annotations": null, "audio": null, "function_call": null, "tool_calls": null}}], "created": null, "model": null, "object": null, "moderation": null, "service_tier": null, "system_fingerprint": null, "usage": null});
        assert_eq!(c.0, expected);
        assert_eq!(
            py::json::dumps(&c.0),
            py::json::dumps(&expected),
            "field order must match model_dump"
        );
    }

    #[test]
    fn status_error_message_matches_openai_python() {
        let e = status_error(400, r#"{"error": {"message": "bad n", "code": null}}"#);
        assert_eq!(
            e.message,
            "Error code: 400 - {'error': {'message': 'bad n', 'code': None}}"
        );
    }
}
