//! Port of upstream `optillm/plugins/json_plugin.py`.
//!
//! Without a JSON schema (or on any error) upstream falls back to a plain
//! completion, which is ported exactly. With a schema upstream ignores the
//! requested model and generates locally with Qwen2.5-Coder-0.5B through
//! `outlines`; optillm-rs has no local engine and instead asks the provider
//! for structured output with the same schema (`response_format`
//! `json_schema`, strict), returning the provider's JSON.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

/// `extract_schema_from_response_format`.
fn extract_schema(response_format: Option<&Value>) -> Option<String> {
    let rf = response_format?.as_object()?;
    if rf.get("type").and_then(|t| t.as_str()) != Some("json_schema") {
        return None;
    }
    let data = rf.get("json_schema").cloned().unwrap_or(json!({}));
    match &data {
        Value::Object(m) if m.contains_key("schema") => Some(py::json::dumps(&m["schema"])),
        _ => Some(py::json::dumps(&data)),
    }
}

async fn plain(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let r = ctx
        .create(json!({"model": ctx.model, "messages": [msg("system", system_prompt), msg("user", initial_query)]}))
        .await?;
    let content = r.message(0).and_then(|m| m.get("content")).cloned();
    let tokens = r.completion_tokens()?;
    Ok((
        content.ok_or_else(|| anyhow!("list index out of range"))?,
        tokens,
    ))
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let Some(schema) = extract_schema(ctx.request_config.get("response_format")) else {
        return plain(system_prompt, initial_query, ctx).await;
    };
    let structured: Result<(Value, i64)> = async {
        let schema_value: Value = serde_json::from_str(&schema)?;
        let r = ctx
            .create(json!({
                "model": ctx.model,
                "messages": [msg("user", initial_query)],
                "response_format": {"type": "json_schema", "json_schema": {"name": "DynamicModel", "schema": schema_value, "strict": true}}
            }))
            .await?;
        let text = r.content_at(0)?.ok_or_else(|| anyhow!("empty structured output"))?;
        let parsed: Value = serde_json::from_str(&text)?;
        let out = match parsed {
            Value::Object(_) => py::json::dumps(&parsed),
            other => py::json::to_str(&other),
        };
        Ok((json!(out), r.completion_tokens()?))
    }
    .await;
    match structured {
        Ok(v) => Ok(v),
        Err(_) => plain(system_prompt, initial_query, ctx).await,
    }
}
