//! Approach registry and executors, ported from upstream `server.py`
//! (`execute_single_approach`, `execute_combined_approaches`,
//! `execute_parallel_approaches`, `execute_n_times`).
//!
//! Approach results are `serde_json::Value`s so they can carry the same
//! dynamic shapes as upstream's Python return values: a string, a list of
//! strings (n > 1) or a full chat-completion dict.

use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use futures::future::BoxFuture;
use serde_json::{json, Map, Value};

use crate::client::{ChatCompletion, SharedClient};
use crate::config::ServerConfig;
use crate::conversation_logger::ConversationLogger;
use crate::parse::{self, Operation};
use crate::py::random::PyRandom;

pub mod bon;
pub mod cot_reflection;
pub mod leap;
pub mod mcts;
pub mod moa;
pub mod plansearch;
pub mod pvg;
pub mod reread;
pub mod rstar;
pub mod rto;
pub mod self_consistency;

/// Upstream `known_approaches`.
pub const KNOWN_APPROACHES: [&str; 15] = [
    "none",
    "mcts",
    "bon",
    "moa",
    "rto",
    "z3",
    "self_consistency",
    "pvg",
    "rstar",
    "cot_reflection",
    "plansearch",
    "leap",
    "re2",
    "cepo",
    "mars",
];

/// SLUGs of the plugins shipped in upstream `optillm/plugins` (v0.4.0).
pub const PLUGIN_SLUGS: [&str; 17] = [
    "executecode",
    "memory",
    "coc",
    "web_search",
    "genselect",
    "deep_research",
    "privacy",
    "compact",
    "deepthink",
    "proxy",
    "spl",
    "router",
    "longcepo",
    "readurls",
    "majority_voting",
    "json",
    "mcp",
];

/// True when `name` is a known approach or plugin slug.
pub fn is_approach(name: &str) -> bool {
    KNOWN_APPROACHES.contains(&name) || PLUGIN_SLUGS.contains(&name)
}

/// Everything an approach needs, mirroring the arguments upstream threads
/// through `execute_single_approach`.
#[derive(Clone)]
pub struct Ctx {
    pub client: SharedClient,
    pub model: String,
    /// Per-request config (the request body minus explicitly handled keys).
    pub request_config: Map<String, Value>,
    pub config: Arc<ServerConfig>,
    pub rng: Arc<Mutex<PyRandom>>,
    pub request_id: Option<String>,
    /// Original (tag-stripped) request messages for passthrough approaches.
    pub messages: Option<Vec<Value>>,
    pub logger: Option<Arc<ConversationLogger>>,
}

impl Ctx {
    /// `client.chat.completions.create(**provider_request)` plus the
    /// `conversation_logger.log_provider_call` that upstream does when a
    /// request id is present.
    pub async fn create(&self, provider_request: Value) -> Result<ChatCompletion> {
        let result = self.client.create(provider_request.clone()).await;
        match result {
            Ok(resp) => {
                if let (Some(logger), Some(id)) = (&self.logger, &self.request_id) {
                    logger.log_provider_call(id, &provider_request, &resp.0);
                }
                Ok(resp)
            }
            Err(e) => Err(anyhow!(e)),
        }
    }

    /// `request_config.get(key, default)` for integers.
    pub fn config_i64(&self, key: &str, default: i64) -> Value {
        match self.request_config.get(key) {
            Some(v) => v.clone(),
            None => json!(default),
        }
    }

    /// `request_config.get('max_tokens', default)`.
    pub fn max_tokens(&self, default: i64) -> Value {
        self.config_i64("max_tokens", default)
    }

    pub fn rng(&self) -> std::sync::MutexGuard<'_, PyRandom> {
        self.rng.lock().expect("rng poisoned")
    }
}

/// Python `min(value, cap)` for a JSON number taken from request config
/// (the first argument wins ties).
pub fn min_tokens(value: &Value, cap: i64) -> Value {
    match value.as_f64() {
        Some(v) if v <= cap as f64 => value.clone(),
        _ => json!(cap),
    }
}

/// Python `min(a, b)`: `b` only replaces `a` when strictly smaller (NaN-safe the same way).
pub fn py_min(a: f64, b: f64) -> f64 {
    if b < a {
        b
    } else {
        a
    }
}

/// Python `max(a, b)`: `b` only replaces `a` when strictly greater.
pub fn py_max(a: f64, b: f64) -> f64 {
    if b > a {
        b
    } else {
        a
    }
}

/// Result of one approach: (response, completion_tokens).
pub type ApproachResult = Result<(Value, i64)>;

/// `execute_single_approach`.
pub fn execute_single_approach<'a>(
    approach: &'a str,
    system_prompt: &'a str,
    initial_query: &'a str,
    ctx: &'a Ctx,
) -> BoxFuture<'a, ApproachResult> {
    Box::pin(async move {
        match approach {
            "none" => none_approach(system_prompt, initial_query, ctx).await,
            "bon" => {
                bon::best_of_n_sampling(system_prompt, initial_query, ctx, ctx.config.best_of_n)
                    .await
            }
            "moa" => moa::mixture_of_agents(system_prompt, initial_query, ctx).await,
            "rto" => rto::round_trip_optimization(system_prompt, initial_query, ctx).await,
            "self_consistency" => {
                self_consistency::advanced_self_consistency_approach(
                    system_prompt,
                    initial_query,
                    ctx,
                )
                .await
            }
            "cot_reflection" => {
                cot_reflection::cot_reflection(
                    system_prompt,
                    initial_query,
                    ctx,
                    ctx.config.return_full_response,
                )
                .await
            }
            "re2" => reread::re2_approach(system_prompt, initial_query, ctx, ctx.config.n).await,
            "mcts" => mcts::chat_with_mcts(system_prompt, initial_query, ctx).await,
            "pvg" => pvg::inference_time_pv_game(system_prompt, initial_query, ctx).await,
            "rstar" => rstar::rstar(system_prompt, initial_query, ctx).await,
            "plansearch" => {
                plansearch::plansearch(system_prompt, initial_query, ctx, ctx.config.n).await
            }
            "leap" => leap::leap(system_prompt, initial_query, ctx).await,
            other if KNOWN_APPROACHES.contains(&other) || PLUGIN_SLUGS.contains(&other) => Err(
                anyhow!("Approach '{}' is not yet implemented in optillm-rs", other),
            ),
            other => Err(anyhow!("Unknown approach: {}", other)),
        }
    })
}

/// The `none` approach: pass the request straight through.
async fn none_approach(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let mut kwargs = ctx.request_config.clone();
    kwargs.shift_remove("stream");
    let messages = match &ctx.messages {
        Some(m) if !m.is_empty() => m.clone(),
        _ => parse::messages_from_prompt(system_prompt, initial_query),
    };
    // Strip 'none-' prefix from model if present.
    let model = ctx.model.strip_prefix("none-").unwrap_or(&ctx.model);
    let normalized = parse::normalize_message_content(&messages);
    let mut provider_request = Map::new();
    provider_request.insert("model".into(), json!(model));
    provider_request.insert("messages".into(), Value::Array(normalized));
    for (k, v) in kwargs {
        provider_request.insert(k, v);
    }
    let response = ctx
        .create(Value::Object(provider_request))
        .await
        .map_err(|e| {
            if let (Some(logger), Some(id)) = (&ctx.logger, &ctx.request_id) {
                logger.log_error(id, &format!("Error in none approach: {}", e));
            }
            e
        })?;
    Ok((response.into_value(), 0))
}

/// Text an approach's output becomes when fed to the next pipeline stage
/// (Python formats non-strings with `str()`).
pub fn as_query(v: &Value) -> String {
    crate::py::json::to_str(v)
}

/// `execute_combined_approaches` (the `&` pipeline).
async fn execute_combined_approaches(
    approaches: &[String],
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let mut final_response = Value::String(initial_query.to_string());
    let mut total_tokens = 0;
    for approach in approaches {
        let query = as_query(&final_response);
        let (response, tokens) =
            execute_single_approach(approach, system_prompt, &query, ctx).await?;
        final_response = response;
        total_tokens += tokens;
    }
    Ok((final_response, total_tokens))
}

/// `execute_parallel_approaches` (the `|` fan-out).
async fn execute_parallel_approaches(
    approaches: &[String],
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let futures = approaches
        .iter()
        .map(|a| execute_single_approach(a, system_prompt, initial_query, ctx));
    let results = futures::future::join_all(futures).await;
    let mut responses = Vec::new();
    let mut total = 0;
    for r in results {
        let (resp, tokens) = r?;
        responses.push(resp);
        total += tokens;
    }
    Ok((Value::Array(responses), total))
}

/// `execute_n_times`.
pub async fn execute_n_times(
    n: i64,
    approaches: &[String],
    operation: Operation,
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let mut responses: Vec<Value> = Vec::new();
    let mut total_tokens = 0;
    // Combined approaches are called without request_id / messages upstream.
    let mut combined_ctx = ctx.clone();
    combined_ctx.request_id = None;
    combined_ctx.messages = None;
    for _ in 0..n.max(0) {
        let (response, tokens) = match operation {
            Operation::Single => {
                execute_single_approach(&approaches[0], system_prompt, initial_query, ctx).await?
            }
            Operation::And => {
                execute_combined_approaches(approaches, system_prompt, initial_query, &combined_ctx)
                    .await?
            }
            Operation::Or => {
                execute_parallel_approaches(approaches, system_prompt, initial_query, &combined_ctx)
                    .await?
            }
        };
        match response {
            Value::Array(items) => responses.extend(items),
            other => responses.push(other),
        }
        total_tokens += tokens;
    }
    if n == 1 && responses.len() == 1 {
        return Ok((responses.pop().unwrap(), total_tokens));
    }
    Ok((Value::Array(responses), total_tokens))
}
