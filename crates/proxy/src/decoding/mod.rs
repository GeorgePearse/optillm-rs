//! Emulation of upstream's local-inference engine (`optillm/inference.py`)
//! for the specialised decoding approaches, against an OpenAI-compatible
//! backend that exposes logprobs and assistant prefill (e.g. vLLM).
//!
//! Upstream only offers `decoding=...` when `OPTILLM_API_KEY` is set and it
//! loads a PyTorch model in-process; with a remote provider the `decoding`
//! kwarg is a TypeError (which the normal client reproduces). optillm-rs ships
//! no PyTorch engine, so in `OPTILLM_API_KEY` mode it wraps the configured
//! backend in [`LocalEmulationClient`] instead:
//!
//! * `cot_decoding`: same algorithm (top-k first tokens, greedy continuation,
//!   confidence = mean p1 - p2, path aggregation); k is capped at the API's 20
//!   top logprobs; num_beams / length_penalty / no_repeat_ngram_size are not
//!   available through the API and are ignored.
//! * `thinkdeeper`: same control loop (minimum thinking via thought-switch
//!   insertion, forced end-of-thinking at the budget) run chunk-wise with
//!   `stop=[end_think_token]` instead of token by token, so `max_thoughts` is
//!   only checked between chunks.
//! * `deepconf`: same warmup / threshold / consensus / weighted-vote logic
//!   with token confidence from the API's top-k logprobs; online early
//!   termination is applied post hoc (the trace is cut where upstream would
//!   have stopped), so the backend still generates the full trace.
//! * `entropy_decoding`: logits-only approximation, one API call per token,
//!   entropy / varentropy from the top-20 logprobs; attention-based metrics
//!   do not exist over the API and are taken as 0, and the model-specific
//!   clarification token is not inserted.
//! * `autothink`: not available (needs an in-process classifier and steering
//!   vectors on hidden states).

mod cot;
mod deepconf;
mod entropy;
mod thinkdeeper;

use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Map, Value};

use crate::client::{ChatCompletion, LlmClient, LlmError, SharedClient};
use crate::py::random::PyRandom;

/// Keyword arguments `InferenceClient.Completions.create` takes beyond OpenAI's.
const LOCAL_KWARGS: [&str; 22] = [
    "active_adapter",
    "decoding",
    "k",
    "num_beams",
    "length_penalty",
    "no_repeat_ngram_size",
    "early_stopping",
    "aggregate_paths",
    "top_k",
    "min_p",
    "reasoning_effort",
    "thought_switch_tokens",
    "min_thinking_tokens",
    "max_thinking_tokens",
    "max_thoughts",
    "prefill",
    "start_think_token",
    "end_think_token",
    "variant",
    "warmup_samples",
    "consensus_threshold",
    "max_traces",
];

/// Wraps the backend client the way upstream's `InferenceClient` replaces it.
pub struct LocalEmulationClient {
    inner: SharedClient,
    rng: Mutex<PyRandom>,
}

impl LocalEmulationClient {
    pub fn new(inner: SharedClient, seed: Option<u64>) -> Self {
        Self {
            inner,
            rng: Mutex::new(
                seed.map(PyRandom::new)
                    .unwrap_or_else(PyRandom::from_entropy),
            ),
        }
    }
}

/// A request split into the parts the decoders need.
pub struct Request {
    pub model: String,
    pub messages: Vec<Value>,
    pub params: Map<String, Value>,
}

impl Request {
    pub fn f64(&self, k: &str, default: f64) -> f64 {
        self.params
            .get(k)
            .and_then(|v| v.as_f64())
            .unwrap_or(default)
    }

    pub fn i64(&self, k: &str, default: i64) -> i64 {
        self.params
            .get(k)
            .and_then(|v| v.as_i64())
            .unwrap_or(default)
    }

    pub fn opt_i64(&self, k: &str) -> Option<i64> {
        self.params.get(k).and_then(|v| v.as_i64())
    }

    pub fn str(&self, k: &str, default: &str) -> String {
        self.params
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    }

    pub fn bool(&self, k: &str, default: bool) -> bool {
        self.params
            .get(k)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }
}

/// Backend access for the decoders.
pub struct Backend<'a> {
    pub client: &'a dyn LlmClient,
    pub model: &'a str,
    pub rng: &'a Mutex<PyRandom>,
}

/// How a partial assistant message is continued (`$OPTILLM_PREFILL_STYLE`).
fn prefill_messages(messages: &[Value], prefix: &str) -> (Vec<Value>, Map<String, Value>) {
    let style = std::env::var("OPTILLM_PREFILL_STYLE").unwrap_or_else(|_| "vllm".into());
    let mut msgs = messages.to_vec();
    let mut extra = Map::new();
    if prefix.is_empty() && style != "deepseek" {
        return (msgs, extra);
    }
    let mut assistant = json!({"role": "assistant", "content": prefix});
    match style.as_str() {
        "deepseek" => {
            assistant["prefix"] = json!(true);
        }
        "none" => {}
        _ => {
            extra.insert("continue_final_message".into(), json!(true));
            extra.insert("add_generation_prompt".into(), json!(false));
        }
    }
    msgs.push(assistant);
    (msgs, extra)
}

impl<'a> Backend<'a> {
    /// One completion continuing `prefix` as the assistant's message.
    pub async fn complete(
        &self,
        messages: &[Value],
        prefix: &str,
        params: Value,
    ) -> Result<ChatCompletion> {
        let (msgs, extra) = prefill_messages(messages, prefix);
        let mut body = Map::new();
        body.insert("model".into(), json!(self.model));
        body.insert("messages".into(), Value::Array(msgs));
        if let Value::Object(p) = params {
            for (k, v) in p {
                body.insert(k, v);
            }
        }
        if !extra.is_empty() {
            body.insert("extra_body".into(), Value::Object(extra));
        }
        self.client
            .create(Value::Object(body))
            .await
            .map_err(|e| anyhow!(e))
    }
}

/// `choice.logprobs.content` as (token, [(token, logprob)...]) entries.
pub fn token_logprobs(resp: &ChatCompletion) -> Vec<(String, f64, Vec<f64>)> {
    let Some(content) = resp
        .choices()
        .first()
        .and_then(|c| c.get("logprobs"))
        .and_then(|l| l.get("content"))
        .and_then(|c| c.as_array())
    else {
        return Vec::new();
    };
    content
        .iter()
        .map(|t| {
            let token = t
                .get("token")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let lp = t.get("logprob").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let tops: Vec<f64> = t
                .get("top_logprobs")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.get("logprob").and_then(|v| v.as_f64()))
                        .collect()
                })
                .unwrap_or_default();
            (token, lp, tops)
        })
        .collect()
}

fn usage_completion_tokens(resp: &ChatCompletion) -> i64 {
    resp.usage()
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

fn usage_prompt_tokens(resp: &ChatCompletion) -> i64 {
    resp.usage()
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// Upstream's local `ChatCompletion.model_dump()`.
fn build_completion(
    model: &str,
    content: &str,
    logprobs: Option<Value>,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> ChatCompletion {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let message = match logprobs {
        Some(lp) => json!({"role": "assistant", "content": content, "logprobs": lp}),
        None => json!({"role": "assistant", "content": content}),
    };
    ChatCompletion(json!({
        "id": format!("chatcmpl-{}", now),
        "object": "chat.completion",
        "created": (now / 1000) as i64,
        "model": model,
        "choices": [{"index": 0, "message": message, "finish_reason": "stop"}],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
            "completion_tokens_details": {"reasoning_tokens": crate::parse::count_reasoning_tokens(content)}
        }
    }))
}

fn split_request(body: Value) -> Result<Request> {
    let mut obj = match crate::client::merge_extra_body(body) {
        Value::Object(o) => o,
        _ => return Err(anyhow!("request body must be an object")),
    };
    let model = obj
        .shift_remove("model")
        .and_then(|m| m.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    let messages = obj
        .shift_remove("messages")
        .and_then(|m| m.as_array().cloned())
        .unwrap_or_default();
    Ok(Request {
        model,
        messages,
        params: obj,
    })
}

fn to_llm(e: anyhow::Error) -> LlmError {
    match e.downcast::<LlmError>() {
        Ok(le) => le,
        Err(e) => LlmError {
            status: None,
            message: e.to_string(),
            body: None,
        },
    }
}

#[async_trait]
impl LlmClient for LocalEmulationClient {
    async fn create(&self, body: Value) -> std::result::Result<ChatCompletion, LlmError> {
        let req = split_request(body).map_err(to_llm)?;
        if req.params.get("stream").and_then(|v| v.as_bool()) == Some(true) {
            return Err(LlmError {
                status: None,
                message: "Streaming is not yet supported".into(),
                body: None,
            });
        }
        let decoding = req
            .params
            .get("decoding")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let backend = Backend {
            client: self.inner.as_ref(),
            model: &req.model,
            rng: &self.rng,
        };
        let out = match decoding.as_deref() {
            None | Some("") => {
                // Standard generation: forward the OpenAI-known parameters only
                // (the local engine accepts and ignores any other kwargs).
                let mut body = Map::new();
                body.insert("model".into(), json!(req.model));
                body.insert("messages".into(), Value::Array(req.messages.clone()));
                for (k, v) in &req.params {
                    if !LOCAL_KWARGS.contains(&k.as_str()) && crate::client::is_create_param(k) {
                        body.insert(k.clone(), v.clone());
                    }
                }
                return self.inner.create(Value::Object(body)).await;
            }
            Some("cot_decoding") => cot::cot_decode(&backend, &req).await,
            Some("thinkdeeper") => thinkdeeper::thinkdeeper_decode(&backend, &req).await,
            Some("deepconf") => deepconf::deepconf_decode(&backend, &req).await,
            Some("entropy_decoding") => entropy::entropy_decode(&backend, &req).await,
            Some("autothink") => Err(anyhow!(
                "autothink needs a local model (complexity classifier + steering vectors on hidden states); optillm-rs has no in-process inference engine"
            )),
            Some(other) => Err(anyhow!("Unknown specialized decoding approach: {}", other)),
        };
        out.map_err(to_llm)
    }

    async fn list_models(&self) -> std::result::Result<Value, LlmError> {
        self.inner.list_models().await
    }

    fn base_url(&self) -> String {
        self.inner.base_url()
    }

    fn api_key(&self) -> String {
        self.inner.api_key()
    }
}

/// Wrap a backend for `OPTILLM_API_KEY` (local-inference) mode.
pub fn wrap(inner: SharedClient, seed: Option<u64>) -> SharedClient {
    Arc::new(LocalEmulationClient::new(inner, seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Backend that answers from a closure and records request bodies.
    struct Fake {
        calls: Mutex<Vec<Value>>,
        reply: Box<dyn Fn(&Value) -> Value + Send + Sync>,
    }

    #[async_trait]
    impl LlmClient for Fake {
        async fn create(&self, body: Value) -> std::result::Result<ChatCompletion, LlmError> {
            let body = crate::client::merge_extra_body(body);
            self.calls.lock().unwrap().push(body.clone());
            Ok(ChatCompletion::from_raw((self.reply)(&body)))
        }
        async fn list_models(&self) -> std::result::Result<Value, LlmError> {
            Ok(json!({}))
        }
        fn base_url(&self) -> String {
            String::new()
        }
        fn api_key(&self) -> String {
            String::new()
        }
    }

    fn lp(token: &str, tops: &[(&str, f64)]) -> Value {
        json!({"token": token, "logprob": tops[0].1.ln(), "bytes": null,
               "top_logprobs": tops.iter().map(|(t, p)| json!({"token": t, "logprob": p.ln()})).collect::<Vec<_>>()})
    }

    #[tokio::test]
    async fn cot_decoding_aggregates_paths() {
        let fake = Arc::new(Fake {
            calls: Mutex::new(Vec::new()),
            reply: Box::new(|body| {
                if body["max_tokens"] == 1 {
                    return json!({"choices": [{"message": {"content": "The"}, "finish_reason": "length",
                        "logprobs": {"content": [lp("The", &[("The", 0.5), ("It", 0.3), ("391", 0.2)])]}}],
                        "usage": {"prompt_tokens": 7, "completion_tokens": 1, "total_tokens": 8}});
                }
                let prefix = body["messages"].as_array().unwrap().last().unwrap()["content"]
                    .as_str()
                    .unwrap()
                    .to_string();
                let (text, conf) = match prefix.as_str() {
                    "The" => (" answer is 391", 0.9),
                    "It" => (" is 391", 0.6),
                    _ => ("", 0.55),
                };
                json!({"choices": [{"message": {"content": text}, "finish_reason": "stop",
                    "logprobs": {"content": [lp("x", &[("x", conf), ("y", 1.0 - conf)])]}}],
                    "usage": {"prompt_tokens": 8, "completion_tokens": 3, "total_tokens": 11}})
            }),
        });
        let client = wrap(fake.clone(), Some(0));
        let resp = client
            .create(json!({"model": "m", "messages": [{"role": "user", "content": "17*23?"}], "decoding": "cot_decoding", "k": 3}))
            .await
            .unwrap();
        assert_eq!(resp.content(0).as_deref(), Some("The answer is 391"));
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0]["top_logprobs"], 3);
        assert_eq!(calls[1]["continue_final_message"], true);
        assert!(calls.iter().all(|c| c.get("decoding").is_none()));
    }

    #[tokio::test]
    async fn thinkdeeper_inserts_thought_switch_below_minimum() {
        let fake = Arc::new(Fake {
            calls: Mutex::new(Vec::new()),
            reply: Box::new(|body| {
                let stop = body.get("stop").is_some();
                let n = if stop { 5 } else { 2 };
                json!({"choices": [{"message": {"content": if stop { "think more" } else { "391" }}, "finish_reason": "stop"}],
                       "usage": {"prompt_tokens": 4, "completion_tokens": n, "total_tokens": 4 + n}})
            }),
        });
        let client = wrap(fake.clone(), Some(0));
        let resp = client
            .create(json!({"model": "m", "messages": [{"role": "user", "content": "q"}], "decoding": "thinkdeeper",
                           "min_thinking_tokens": 8, "max_thinking_tokens": 50}))
            .await
            .unwrap();
        let text = resp.content(0).unwrap();
        assert!(text.starts_with("<think>\n"));
        assert!(text.ends_with("</think>391"));
        // One switch phrase inserted because the first chunk (5 tokens) was below 8.
        assert_eq!(fake.calls.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn plain_requests_drop_local_kwargs() {
        let fake = Arc::new(Fake {
            calls: Mutex::new(Vec::new()),
            reply: Box::new(|_| json!({"choices": [{"message": {"content": "ok"}}]})),
        });
        let client = wrap(fake.clone(), None);
        client
            .create(json!({"model": "m", "messages": [], "temperature": 0.2, "mcts_simulations": 3, "k": 5}))
            .await
            .unwrap();
        let calls = fake.calls.lock().unwrap();
        assert_eq!(
            calls[0],
            json!({"model": "m", "messages": [], "temperature": 0.2})
        );
    }
}
