//! Port of upstream `optillm/plugins/router_plugin.py`.
//!
//! The approach is predicted by upstream's ModernBERT-large classifier, run
//! through `py_helper.py` (needs torch + transformers and the HF hub). The
//! chosen approach then runs natively with upstream's call defaults (the
//! router passes no request config, so every approach uses its own
//! defaults). Any failure, including the model being unavailable, takes
//! upstream's fallback: one direct completion.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use super::pyhelper;
use crate::approaches::{self, ApproachResult, Ctx};
use crate::config::ServerConfig;
use crate::parse::msg;

async fn routed(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let r = pyhelper::call(
        json!({"op": "router", "system_prompt": system_prompt, "initial_query": initial_query}),
        1800,
    )
    .await?;
    let approach = r
        .get("approach")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow!(r
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("router failed")
                .to_string())
        })?
        .to_string();
    if approach == "none" {
        return direct(system_prompt, initial_query, ctx).await;
    }
    let mut defaults = ServerConfig {
        random_seed: ctx.config.random_seed,
        ..ServerConfig::default()
    };
    defaults.cepo_overrides = ctx.config.cepo_overrides.clone();
    let mut sub = ctx.clone();
    sub.request_config = Map::new();
    sub.config = Arc::new(defaults);
    sub.request_id = None;
    approaches::execute_single_approach(&approach, system_prompt, initial_query, &sub).await
}

async fn direct(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> Result<(Value, i64)> {
    let r = ctx
        .create(json!({"model": ctx.model, "messages": [msg("system", system_prompt), msg("user", initial_query)]}))
        .await?;
    if r.choices().is_empty() {
        return Err(anyhow!("list index out of range"));
    }
    let content = r
        .message(0)
        .and_then(|m| m.get("content"))
        .cloned()
        .unwrap_or(Value::Null);
    Ok((content, r.completion_tokens()?))
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    match routed(system_prompt, initial_query, ctx).await {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::warn!(
                "Error in router plugin: {}. Falling back to direct model usage.",
                e
            );
            direct(system_prompt, initial_query, ctx).await
        }
    }
}
