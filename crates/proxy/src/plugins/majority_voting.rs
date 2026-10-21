//! Port of upstream `optillm/plugins/majority_voting_plugin.py`.

use anyhow::Result;
use regex::Regex;
use serde_json::{json, Value};

use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

fn remove_think(s: &str) -> String {
    Regex::new(r"(?s)<think>.*?</think>")
        .unwrap()
        .replace_all(s, "")
        .to_string()
}

/// `normalize_response`.
pub fn normalize_response(response: &str) -> String {
    if response.is_empty() {
        return String::new();
    }
    let r = remove_think(response);
    let r = py::strings::strip(&r).to_lowercase();
    let r = r.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    py::strings::split_whitespace(r).join(" ")
}

/// `extract_final_answer`.
pub fn extract_final_answer(response: &str) -> String {
    if response.is_empty() {
        return response.to_string();
    }
    let r = py::strings::strip(&remove_think(response)).to_string();
    let patterns = [
        r"(?im)(?:final answer|answer):\s*(.+?)(?:\n|$)",
        r"(?im)(?:the answer is|answer is)\s*(.+?)(?:\n|$)",
        r"(?im)###\s*(.+?)(?:\n|$)",
        r"(?im)^([A-E])\b",
        r"(?im)\b([A-E])\b\s*$",
    ];
    for p in patterns {
        if let Some(c) = Regex::new(p).unwrap().captures(&r) {
            return py::strings::strip(&c[1]).to_string();
        }
    }
    r
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let rc = &ctx.request_config;
    let k = rc.get("k").cloned().unwrap_or(json!(8));
    let temperature = rc.get("temperature").cloned().unwrap_or(json!(0.6));
    let max_tokens = rc.get("max_tokens").cloned().unwrap_or(json!(4096));
    let messages = json!([msg("system", system_prompt), msg("user", initial_query)]);

    let mut candidates: Vec<Value> = Vec::new();
    let mut total_tokens: i64 = 0;
    let parallel: Result<()> = async {
        let response = ctx
            .create(json!({"model": ctx.model, "messages": messages, "n": k, "temperature": temperature, "max_tokens": max_tokens}))
            .await?;
        candidates = response
            .choices()
            .iter()
            .map(|c| c.get("message").and_then(|m| m.get("content")).cloned().unwrap_or(Value::Null))
            .collect();
        total_tokens = response.completion_tokens()?;
        Ok(())
    }
    .await;
    if parallel.is_err() {
        for _ in 0..k.as_i64().unwrap_or(0) {
            let r: Result<()> = async {
                let response = ctx
                    .create(json!({"model": ctx.model, "messages": messages, "temperature": temperature, "max_tokens": max_tokens}))
                    .await?;
                let content = response
                    .choices()
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("list index out of range"))?
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .cloned()
                    .unwrap_or(Value::Null);
                candidates.push(content);
                total_tokens += response.completion_tokens()?;
                Ok(())
            }
            .await;
            let _ = r;
        }
    }
    if candidates.is_empty() {
        return Ok((json!("Error: Could not generate any candidates"), 0));
    }

    let mut votes: Vec<(String, usize, Value)> = Vec::new();
    for c in &candidates {
        let text = c.as_str().unwrap_or("");
        let normalized = normalize_response(&extract_final_answer(text));
        if normalized.is_empty() {
            continue;
        }
        match votes.iter_mut().find(|(n, _, _)| *n == normalized) {
            Some(v) => v.1 += 1,
            None => votes.push((normalized, 1, c.clone())),
        }
    }
    let best = votes
        .iter()
        .fold(None::<&(String, usize, Value)>, |acc, v| match acc {
            Some(a) if a.1 >= v.1 => Some(a),
            _ => Some(v),
        });
    match best {
        Some(b) => Ok((b.2.clone(), total_tokens)),
        None => Ok((candidates[0].clone(), total_tokens)),
    }
}
