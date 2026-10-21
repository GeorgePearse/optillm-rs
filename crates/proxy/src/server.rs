//! The OpenAI-compatible HTTP surface, ported from upstream `server.py`:
//! `/v1/chat/completions` (incl. streaming), `/v1/models`, `/health`, the
//! optional bearer-key check and batch mode.

use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Map, Value};

use crate::approaches::{self, is_approach, Ctx};
use crate::batching::{BatchRequest, Batcher};
use crate::client::{Flavor, OpenAiClient, SharedClient};
use crate::config::ServerConfig;
use crate::conversation_logger::ConversationLogger;
use crate::parse::{self, Operation};
use crate::py;
use crate::py::random::PyRandom;

/// Shared server state.
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub default_client: Option<SharedClient>,
    pub api_key: Option<String>,
    pub logger: Arc<ConversationLogger>,
    pub batcher: Option<Arc<Batcher>>,
}

/// Upstream `get_config()`: choose the provider client from the environment.
pub fn build_default_client(config: &ServerConfig) -> (Option<SharedClient>, Option<String>) {
    let env = |k: &str| {
        std::env::var(k)
            .ok()
            .filter(|v| !v.is_empty() || k.ends_with("_KEY"))
    };
    let ssl_cert = Some(config.ssl_cert_path.as_str());
    if std::env::var("OPTILLM_API_KEY").is_ok() {
        tracing::warn!(
            "OPTILLM_API_KEY is set: upstream switches to its local PyTorch inference engine, \
             which optillm-rs does not ship; falling back to the next configured provider"
        );
    }
    if let Some(key) = env("CEREBRAS_API_KEY") {
        let base = if config.base_url.is_empty() {
            std::env::var("CEREBRAS_BASE_URL").unwrap_or_else(|_| "https://api.cerebras.ai".into())
        } else {
            config.base_url.clone()
        };
        let client =
            OpenAiClient::with_flavor(&key, base, Flavor::Cerebras, config.ssl_verify, ssl_cert);
        return (Some(Arc::new(client)), Some(key));
    }
    if let Some(key) = env("OPENAI_API_KEY") {
        let base = if config.base_url.is_empty() {
            openai_default_base()
        } else {
            config.base_url.clone()
        };
        let client =
            OpenAiClient::with_flavor(&key, base, Flavor::OpenAi, config.ssl_verify, ssl_cert);
        return (Some(Arc::new(client)), Some(key));
    }
    if let Some(key) = env("AZURE_OPENAI_API_KEY") {
        let api_version = std::env::var("AZURE_API_VERSION").unwrap_or_default();
        let endpoint = std::env::var("AZURE_API_BASE").unwrap_or_default();
        let client = OpenAiClient::with_flavor(
            &key,
            endpoint,
            Flavor::Azure { api_version },
            config.ssl_verify,
            ssl_cert,
        );
        return (Some(Arc::new(client)), Some(key));
    }
    tracing::warn!(
        "No OPENAI_API_KEY / CEREBRAS_API_KEY / AZURE_OPENAI_API_KEY set. Upstream falls back to \
         LiteLLM here; optillm-rs has no LiteLLM router, so requests will fail"
    );
    (None, None)
}

/// openai-python's default base URL (honours `OPENAI_BASE_URL`).
pub fn openai_default_base() -> String {
    std::env::var("OPENAI_BASE_URL")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://api.openai.com/v1".into())
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(proxy))
        .route("/v1/models", get(proxy_models))
        .route("/health", get(health))
        .fallback(not_found)
        .layer(middleware::from_fn_with_state(state.clone(), check_api_key))
        .with_state(state)
}

/// Flask `jsonify(value), status`.
pub fn jsonify(value: &Value, status: StatusCode) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(py::json::flask_jsonify(value)))
        .unwrap()
}

fn werkzeug_error(status: StatusCode, name: &str, description: &str) -> Response {
    let body = format!(
        "<!doctype html>\n<html lang=en>\n<title>{code} {name}</title>\n<h1>{name}</h1>\n<p>{description}</p>\n",
        code = status.as_u16(),
        name = name,
        description = description
    );
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

async fn not_found() -> Response {
    werkzeug_error(
        StatusCode::NOT_FOUND,
        "Not Found",
        "The requested URL was not found on the server. If you entered the URL manually please check your spelling and try again.",
    )
}

/// `@app.before_request check_api_key`.
async fn check_api_key(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let key = &state.config.optillm_api_key;
    if !key.is_empty() && req.uri().path() != "/health" {
        let auth = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        match auth {
            Some(a) if a.starts_with("Bearer ") => {
                let client_key =
                    py::strings::strip(a.split_once("Bearer ").map(|x| x.1).unwrap_or(""));
                if !constant_time_eq(client_key.as_bytes(), key.as_bytes()) {
                    return jsonify(
                        &json!({"error": "Invalid API key"}),
                        StatusCode::UNAUTHORIZED,
                    );
                }
            }
            _ => {
                return jsonify(
                    &json!({"error": "Invalid Authorization header. Expected format: 'Authorization: Bearer YOUR_API_KEY'"}),
                    StatusCode::UNAUTHORIZED,
                )
            }
        }
    }
    next.run(req).await
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn health() -> Response {
    jsonify(&json!({"status": "ok"}), StatusCode::OK)
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn sse(body: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(body))
        .unwrap()
}

/// `generate_streaming_response(final_response, model)`.
pub fn generate_streaming_response(final_response: &Value, model: &str) -> String {
    let response_id = format!("chatcmpl-{}", now_ms());
    let created = (now_ms() / 1000) as i64;
    let mut out = String::new();
    let mut chunk = |content: &Value, index: usize| {
        let v = json!({
            "id": response_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "choices": [{"delta": {"role": "assistant", "content": content}, "index": index, "finish_reason": "stop"}],
        });
        out.push_str("data: ");
        out.push_str(&py::json::dumps(&v));
        out.push_str("\n\n");
    };
    match final_response {
        Value::Array(items) => {
            for (i, r) in items.iter().enumerate() {
                chunk(r, i);
            }
        }
        other => chunk(other, 0),
    }
    out.push_str("data: [DONE]\n\n");
    out
}

/// `generate_streaming_completion(completion, model, include_usage)`.
pub fn generate_streaming_completion(
    completion: &Value,
    model: &str,
    include_usage: bool,
) -> String {
    let truthy = |v: Option<&Value>| match v {
        None | Some(Value::Null) => false,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::Bool(b)) => *b,
    };
    let response_id = if truthy(completion.get("id")) {
        completion["id"].clone()
    } else {
        json!(format!("chatcmpl-{}", now_ms()))
    };
    let created = if truthy(completion.get("created")) {
        completion["created"].clone()
    } else {
        json!((now_ms() / 1000) as i64)
    };
    let model = if truthy(completion.get("model")) {
        completion["model"].clone()
    } else {
        json!(model)
    };
    let mut out = String::new();
    let mut chunk = |choices: Value, extra: Option<(&str, Value)>| {
        let mut m = Map::new();
        m.insert("id".into(), response_id.clone());
        m.insert("object".into(), json!("chat.completion.chunk"));
        m.insert("created".into(), created.clone());
        m.insert("model".into(), model.clone());
        m.insert("choices".into(), choices);
        if let Some((k, v)) = extra {
            m.insert(k.into(), v);
        }
        out.push_str("data: ");
        out.push_str(&py::json::dumps(&Value::Object(m)));
        out.push_str("\n\n");
    };
    let empty = Vec::new();
    let choices = completion
        .get("choices")
        .and_then(|c| c.as_array())
        .unwrap_or(&empty);
    for choice in choices {
        let message = choice.get("message").cloned().unwrap_or(json!({}));
        let role = if truthy(message.get("role")) {
            message["role"].clone()
        } else {
            json!("assistant")
        };
        let mut delta = Map::new();
        delta.insert("role".into(), role);
        for key in ["content", "reasoning_content", "refusal"] {
            if let Some(v) = message.get(key) {
                if !v.is_null() {
                    delta.insert(key.into(), v.clone());
                }
            }
        }
        if truthy(message.get("tool_calls")) {
            let calls: Vec<Value> = message["tool_calls"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .enumerate()
                .map(|(i, tc)| {
                    let mut m = Map::new();
                    m.insert("index".into(), json!(i));
                    if let Value::Object(o) = tc {
                        for (k, v) in o {
                            m.insert(k, v);
                        }
                    }
                    Value::Object(m)
                })
                .collect();
            delta.insert("tool_calls".into(), Value::Array(calls));
        }
        let index = choice.get("index").cloned().unwrap_or(json!(0));
        let index = if index.is_null() { json!(0) } else { index };
        chunk(
            json!([{"index": index, "delta": Value::Object(delta), "finish_reason": null}]),
            None,
        );
        let finish = if truthy(choice.get("finish_reason")) {
            choice["finish_reason"].clone()
        } else {
            json!("stop")
        };
        chunk(
            json!([{"index": index, "delta": {}, "finish_reason": finish}]),
            None,
        );
    }
    if include_usage && truthy(completion.get("usage")) {
        chunk(json!([]), Some(("usage", completion["usage"].clone())));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

/// Turn approach output into the final response list / string
/// (`tagged_conversation_to_messages` post-processing in `proxy()`).
pub fn untag(response: Value) -> Value {
    match response {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|v| match v {
                    Value::String(s) => Value::String(parse::untag_response(&s)),
                    other => other,
                })
                .collect(),
        ),
        Value::String(s) => Value::String(parse::untag_response(&s)),
        other => other,
    }
}

fn choices_for(response: &Value) -> Vec<Value> {
    match response {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, r)| json!({"index": i, "message": {"role": "assistant", "content": r}, "finish_reason": "stop"}))
            .collect(),
        other => vec![json!({"index": 0, "message": {"role": "assistant", "content": other}, "finish_reason": "stop"})],
    }
}

fn is_full_completion(v: &Value) -> bool {
    v.as_object()
        .map(|o| o.contains_key("choices") && o.contains_key("usage"))
        .unwrap_or(false)
}

fn error_500(msg: &str) -> Response {
    jsonify(&json!({"error": msg}), StatusCode::INTERNAL_SERVER_ERROR)
}

/// Inputs gathered from one chat-completions request.
#[derive(Clone)]
pub struct Prepared {
    pub system_prompt: String,
    pub initial_query: String,
    pub client: Option<SharedClient>,
    pub model: String,
    pub request_config: Map<String, Value>,
    pub approaches: Vec<String>,
    pub operation: Operation,
    pub n: Value,
    pub stream: bool,
    pub optillm_approach: String,
}

pub fn new_ctx(
    state: &AppState,
    p: &Prepared,
    request_id: Option<String>,
    messages: Option<Vec<Value>>,
) -> Result<Ctx, String> {
    let client = p.client.clone().ok_or_else(|| {
        "No LLM provider configured (set OPENAI_API_KEY, CEREBRAS_API_KEY or AZURE_OPENAI_API_KEY)".to_string()
    })?;
    let rng = match state.config.random_seed {
        Some(seed) => PyRandom::new(seed),
        None => PyRandom::from_entropy(),
    };
    Ok(Ctx {
        client,
        model: p.model.clone(),
        request_config: p.request_config.clone(),
        config: state.config.clone(),
        rng: Arc::new(Mutex::new(rng)),
        request_id,
        messages,
        logger: Some(state.logger.clone()),
    })
}

fn n_as_i64(n: &Value) -> Result<i64, String> {
    match n {
        Value::Number(x) if x.is_i64() => Ok(x.as_i64().unwrap()),
        Value::Bool(b) => Ok(*b as i64),
        Value::Null => Err("'NoneType' object cannot be interpreted as an integer".into()),
        Value::Number(_) => Err("'float' object cannot be interpreted as an integer".into()),
        Value::String(_) => Err("'str' object cannot be interpreted as an integer".into()),
        _ => Err("object cannot be interpreted as an integer".into()),
    }
}

/// `POST /v1/chat/completions` (upstream `proxy()`).
async fn proxy(State(state): State<Arc<AppState>>, headers: HeaderMap, body: Bytes) -> Response {
    let is_json_ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| {
            let ct = ct.split(';').next().unwrap_or("").trim();
            ct == "application/json" || (ct.starts_with("application/") && ct.ends_with("+json"))
        })
        .unwrap_or(false);
    if !is_json_ct {
        return werkzeug_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Unsupported Media Type",
            "Did not attempt to load JSON data because the request Content-Type was not &#39;application/json&#39;.",
        );
    }
    let data: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return werkzeug_error(
                StatusCode::BAD_REQUEST,
                "Bad Request",
                "Failed to decode JSON object: invalid JSON",
            )
        }
    };
    let empty = Map::new();
    let obj = data.as_object().unwrap_or(&empty);
    let config = &state.config;

    let bearer_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .filter(|a| a.starts_with("Bearer "))
        .map(|a| py::strings::strip(a.split("Bearer ").nth(1).unwrap_or("")).to_string())
        .unwrap_or_default();

    let stream = obj.get("stream").map(truthy_json).unwrap_or(false);
    let stream_value = obj.get("stream").cloned().unwrap_or(json!(false));
    let messages: Vec<Value> = obj
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    let mut model = match obj.get("model") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => py::json::to_str(other),
        None => config.model.clone(),
    };
    let n = obj.get("n").cloned().unwrap_or(json!(config.n));
    let response_format = obj.get("response_format").cloned().filter(|v| !v.is_null());
    let max_completion_tokens = obj
        .get("max_completion_tokens")
        .cloned()
        .filter(|v| !v.is_null());
    let max_tokens = obj.get("max_tokens").cloned().filter(|v| !v.is_null());
    let include_usage = obj
        .get("stream_options")
        .and_then(|o| o.get("include_usage"))
        .map(truthy_json)
        .unwrap_or(false);

    let explicit = [
        "stream",
        "stream_options",
        "messages",
        "model",
        "n",
        "response_format",
        "max_completion_tokens",
        "max_tokens",
    ];
    let mut request_config: Map<String, Value> = obj
        .iter()
        .filter(|(k, _)| !explicit.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    request_config.insert("stream".into(), stream_value);
    request_config.insert("n".into(), n.clone());
    if let Some(rf) = response_format {
        request_config.insert("response_format".into(), rf);
    }
    if let Some(mct) = max_completion_tokens {
        request_config.insert("max_completion_tokens".into(), mct.clone());
        request_config.insert("max_tokens".into(), mct);
    } else if let Some(mt) = max_tokens {
        request_config.insert("max_tokens".into(), mt);
    }

    let mut optillm_approach = match obj.get("optillm_approach") {
        Some(v) => py::json::to_str(v),
        None => config.approach.clone(),
    };
    let (system_prompt, initial_query, message_approach) = parse::parse_conversation(&messages);
    let passthrough_messages = parse::strip_optillm_approach_tags(&messages);
    if let Some(a) = message_approach.filter(|a| !a.is_empty()) {
        optillm_approach = a;
    }
    if optillm_approach != "auto" {
        model = format!("{}-{}", optillm_approach, model);
    }

    let (operation, approach_list, model) = parse::parse_combined_approach(&model, is_approach);

    let request_id = if state.logger.enabled {
        let mut client_request = Map::new();
        client_request.insert("messages".into(), Value::Array(messages.clone()));
        client_request.insert(
            "model".into(),
            obj.get("model").cloned().unwrap_or(json!(config.model)),
        );
        client_request.insert("stream".into(), json!(stream));
        client_request.insert("n".into(), n.clone());
        for (k, v) in obj {
            if !["messages", "model", "stream", "n"].contains(&k.as_str()) {
                client_request.insert(k.clone(), v.clone());
            }
        }
        let approach_label = if approach_list.len() == 1 {
            approach_list[0].clone()
        } else {
            format!("{}({})", operation.as_str(), approach_list.join(","))
        };
        let id =
            state
                .logger
                .start_conversation(Value::Object(client_request), &approach_label, &model);
        Some(id).filter(|s| !s.is_empty())
    } else {
        None
    };
    tracing::info!(
        "Using approach(es) {:?}, operation {}, with model {}",
        approach_list,
        operation.as_str(),
        model
    );

    let client: Option<SharedClient> = if bearer_token.starts_with("sk-") {
        let base = if config.base_url.is_empty() {
            openai_default_base()
        } else {
            config.base_url.clone()
        };
        Some(Arc::new(OpenAiClient::new(bearer_token.clone(), base)))
    } else {
        state.default_client.clone()
    };

    let prepared = Prepared {
        system_prompt,
        initial_query,
        client,
        model: model.clone(),
        request_config,
        approaches: approach_list.clone(),
        operation,
        n: n.clone(),
        stream,
        optillm_approach,
    };

    if let Some(batcher) = &state.batcher {
        return match batcher.add_request(BatchRequest { prepared }).await {
            Ok(v) => jsonify(&v, StatusCode::OK),
            Err(e) => error_500(&e),
        };
    }

    let finish_error = |msg: String| -> Response {
        if let Some(id) = &request_id {
            state.logger.log_error(id, &msg);
            state.logger.finalize_conversation(id);
        }
        tracing::error!("Error processing request: {}", msg);
        error_500(&msg)
    };
    let finish_ok = |v: &Value| {
        if let Some(id) = &request_id {
            state.logger.log_final_response(id, v);
            state.logger.finalize_conversation(id);
        }
    };

    let contains_none = approach_list.iter().any(|a| a == "none");
    let ctx = match new_ctx(
        &state,
        &prepared,
        request_id.clone(),
        Some(passthrough_messages),
    ) {
        Ok(c) => c,
        Err(e) => return finish_error(e),
    };

    if operation == Operation::Single && approach_list[0] == "none" {
        let result = approaches::execute_single_approach(
            "none",
            &prepared.system_prompt,
            &prepared.initial_query,
            &ctx,
        )
        .await;
        return match result {
            Ok((result, _)) => {
                finish_ok(&result);
                if stream {
                    sse(generate_streaming_completion(
                        &result,
                        &model,
                        include_usage,
                    ))
                } else {
                    jsonify(&result, StatusCode::OK)
                }
            }
            Err(e) => finish_error(e.to_string()),
        };
    } else if (operation == Operation::And || operation == Operation::Or) && contains_none {
        return finish_error("'none' approach cannot be combined with other approaches".into());
    }

    let n_int = match n_as_i64(&n) {
        Ok(v) => v,
        Err(e) => return finish_error(e),
    };
    let (response, completion_tokens) = match approaches::execute_n_times(
        n_int,
        &approach_list,
        operation,
        &prepared.system_prompt,
        &prepared.initial_query,
        &ctx,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return finish_error(e.to_string()),
    };

    if operation == Operation::Single && is_full_completion(&response) {
        finish_ok(&response);
        return if stream {
            sse(generate_streaming_completion(
                &response,
                &model,
                include_usage,
            ))
        } else {
            jsonify(&response, StatusCode::OK)
        };
    }

    let response = untag(response);

    if stream {
        return sse(generate_streaming_response(&response, &model));
    }

    let reasoning_tokens: i64 = match &response {
        Value::String(s) => parse::count_reasoning_tokens(s),
        Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(parse::count_reasoning_tokens)
            .sum(),
        _ => 0,
    };
    let response_data = json!({
        "model": model,
        "choices": choices_for(&response),
        "usage": {
            "completion_tokens": completion_tokens,
            "completion_tokens_details": {"reasoning_tokens": reasoning_tokens}
        }
    });
    finish_ok(&response_data);
    jsonify(&response_data, StatusCode::OK)
}

fn truthy_json(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Process one request inside a batch (upstream `process_batch_requests` body).
pub async fn process_batch_item(state: &AppState, p: &Prepared, i: usize) -> Result<Value, String> {
    let fail = |e: String| format!("Failed to process request {}: {}", i + 1, e);
    if p.stream {
        return Err(fail("Streaming requests cannot be batched".into()));
    }
    let ctx = new_ctx(state, p, None, None).map_err(fail)?;
    let contains_none = p.approaches.iter().any(|a| a == "none");
    let (result, completion_tokens) =
        if p.operation == Operation::Single && p.approaches[0] == "none" {
            approaches::execute_single_approach("none", &p.system_prompt, &p.initial_query, &ctx)
                .await
                .map_err(|e| fail(e.to_string()))?
        } else {
            if (p.operation == Operation::And || p.operation == Operation::Or) && contains_none {
                return Err(fail(
                    "'none' approach cannot be combined with other approaches".into(),
                ));
            }
            let n = n_as_i64(&p.n).map_err(fail)?;
            approaches::execute_n_times(
                n,
                &p.approaches,
                p.operation,
                &p.system_prompt,
                &p.initial_query,
                &ctx,
            )
            .await
            .map_err(|e| fail(e.to_string()))?
        };
    let result = untag(result);
    let ts = now_ms();
    Ok(json!({
        "id": format!("chatcmpl-{}-{}", ts, i),
        "object": "chat.completion",
        "created": (ts / 1000) as i64,
        "model": p.model,
        "choices": choices_for(&result),
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": completion_tokens,
            "total_tokens": completion_tokens
        }
    }))
}

/// `GET /v1/models`.
async fn proxy_models(State(state): State<Arc<AppState>>) -> Response {
    let config = &state.config;
    if config.approach == "proxy" {
        if let Some(models) = crate::plugins::proxy_models().await {
            return jsonify(&models, StatusCode::OK);
        }
    }
    if !config.base_url.is_empty() {
        let Some(key) = state.api_key.clone() else {
            return error_500("Error fetching models: The api_key client option must be set either by passing api_key to the client or by setting the OPENAI_API_KEY environment variable");
        };
        let client = OpenAiClient::new(key, config.base_url.clone());
        use crate::client::LlmClient;
        return match client.list_models().await {
            Ok(v) => jsonify(&v, StatusCode::OK),
            Err(e) => error_500(&format!("Error fetching models: {}", e)),
        };
    }
    jsonify(
        &json!({
            "object": "list",
            "data": [{"id": config.model, "object": "model", "created": 1677610602, "owned_by": "optillm"}]
        }),
        StatusCode::OK,
    )
}

pub fn into_response(r: impl IntoResponse) -> Response {
    r.into_response()
}
