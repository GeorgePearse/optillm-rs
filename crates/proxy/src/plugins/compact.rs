//! Port of upstream `optillm/plugins/compact_plugin.py`.

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Value};

use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

#[rustfmt::skip]
const COMPACT_SYSTEM_PROMPT: &str = concat!(
    "You are a conversation summarizer. Given a conversation history, produce a structured summary.\n",
    "\n",
    "Output ONLY this format, nothing else:\n",
    "\n",
    "<summary>\n",
    "Conversation summary:\n",
    "- Scope: {N} earlier messages compacted (user={U}, assistant={A}).\n",
    "- Key decisions: {list the main decisions or conclusions reached}\n",
    "- User preferences: {any stated preferences or constraints}\n",
    "- Pending work: {any remaining tasks or next steps mentioned}\n",
    "- Key files referenced: {file paths mentioned, if any}\n",
    "- Context: {a concise paragraph capturing the essential context needed to continue}\n",
    "</summary>\n",
    "\n",
    "Rules:\n",
    "- Be specific: include actual values, names, and file paths — not vague references\n",
    "- Be concise: each section should be 1-2 lines maximum\n",
    "- Omit pleasantries, greetings, and filler\n",
    "- The Context paragraph is the most important part — it should capture everything a new assistant would need to pick up where this left off",
);

/// `_get_config(request_config, key, env_var, default)` for numbers.
fn get_config(ctx: &Ctx, key: &str, env_var: &str, default: f64, int: bool) -> f64 {
    if let Some(v) = ctx.request_config.get(key).filter(|v| !v.is_null()) {
        return v.as_f64().unwrap_or(default);
    }
    if let Ok(env_val) = std::env::var(env_var) {
        let parsed = if int {
            py::strings::parse_int(&env_val).map(|i| i as f64)
        } else {
            py::strings::parse_float(&env_val)
        };
        return parsed.unwrap_or(default);
    }
    default
}

/// `estimate_tokens`: tiktoken's gpt-4 encoding (cl100k_base).
pub fn estimate_tokens(text: &str) -> Result<usize> {
    for special in [
        "<|endoftext|>",
        "<|fim_prefix|>",
        "<|fim_middle|>",
        "<|fim_suffix|>",
        "<|endofprompt|>",
    ] {
        if text.contains(special) {
            return Err(anyhow!(
                "Encountered text corresponding to disallowed special token '{}'.",
                special
            ));
        }
    }
    let bpe = tiktoken_rs::cl100k_base().map_err(|e| anyhow!(e.to_string()))?;
    Ok(bpe.encode_ordinary(text).len())
}

/// `parse_tagged_conversation`.
pub fn parse_tagged_conversation(text: &str) -> Vec<(String, String)> {
    let head = Regex::new(r"(?m)^(User:|Assistant:)\s*").unwrap();
    let next = Regex::new(r"(?m)^(User:|Assistant:)").unwrap();
    let mut turns = Vec::new();
    for m in head.captures_iter(text) {
        let role = if &m[1] == "User:" {
            "user"
        } else {
            "assistant"
        };
        let start = m.get(0).unwrap().end();
        let rest = &text[start..];
        let content = match next.find(rest) {
            Some(n) => &rest[..n.start()],
            None => rest,
        };
        turns.push((role.to_string(), py::strings::strip(content).to_string()));
    }
    turns
}

fn reconstruct_tagged(turns: &[(String, String)]) -> String {
    turns
        .iter()
        .map(|(role, content)| {
            format!(
                "{} {}",
                if role == "user" {
                    "User:"
                } else {
                    "Assistant:"
                },
                content
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn context_window(ctx: &Ctx) -> f64 {
    if let Ok(info) = ctx.client.retrieve_model(&ctx.model).await {
        for attr in [
            "context_length",
            "max_context_length",
            "context_window",
            "max_model_length",
            "max_position_embeddings",
        ] {
            match info.get(attr) {
                Some(Value::Null) | None => continue,
                Some(v) => {
                    if let Some(f) = v.as_f64() {
                        return f.trunc();
                    }
                }
            }
        }
    }
    get_config(
        ctx,
        "compact_context_window",
        "COMPACT_CONTEXT_WINDOW",
        128000.0,
        true,
    )
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let window = context_window(ctx).await;
    let threshold = get_config(ctx, "compact_threshold", "COMPACT_THRESHOLD", 0.75, false);
    let keep_recent =
        get_config(ctx, "compact_keep_recent", "COMPACT_KEEP_RECENT", 4.0, true) as i64;

    let token_count = estimate_tokens(initial_query)? as f64;
    let budget = (window * threshold).trunc();
    if token_count < budget {
        return Ok((json!(initial_query), 0));
    }
    let turns = parse_tagged_conversation(initial_query);
    if turns.len() as i64 <= keep_recent {
        return Ok((json!(initial_query), 0));
    }
    let split_idx = (turns.len() as i64 - keep_recent).max(0) as usize;
    let (older, recent) = turns.split_at(split_idx);

    let mut system_content = COMPACT_SYSTEM_PROMPT.to_string();
    if !system_prompt.is_empty() {
        system_content += &format!("\n\nOriginal system context: {}", system_prompt);
    }
    let response = match ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", &system_content), msg("user", &reconstruct_tagged(older))],
            "max_tokens": 2000,
            "temperature": 0.3
        }))
        .await
    {
        Ok(r) => r,
        Err(_) => return Ok((json!(initial_query), 0)),
    };
    let raw = response
        .content_at(0)?
        .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
    let raw = py::strings::strip(&raw).to_string();
    let tokens = response
        .usage()
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);
    let summary = match Regex::new(r"(?s)<summary>(.*?)</summary>")
        .unwrap()
        .captures(&raw)
    {
        Some(c) => py::strings::strip(&c[1]).to_string(),
        None => raw,
    };
    let mut compressed = vec![(
        "user".to_string(),
        format!("[Conversation summary]:\n{}", summary),
    )];
    compressed.extend(recent.iter().cloned());
    Ok((json!(reconstruct_tagged(&compressed)), tokens))
}
