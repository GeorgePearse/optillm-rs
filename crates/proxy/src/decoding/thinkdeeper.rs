//! `thinkdeeper` over chunked API generation; mirrors upstream
//! `optillm/thinkdeeper.py` + `inference.get_effort_profile`.

use anyhow::Result;
use serde_json::{json, Value};

use super::{build_completion, usage_completion_tokens, usage_prompt_tokens, Backend, Request};
use crate::client::ChatCompletion;

/// `get_effort_profile(reasoning_effort, max_tokens)`.
fn effort_profile(effort: &str, max_tokens: i64) -> (i64, i64, i64, Vec<String>) {
    let switches: Vec<String> = ["Wait,", "Alternatively,", "However,", "Additionally,"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (max_pct, max_thoughts) = match effort.to_lowercase().as_str() {
        "medium" => (0.66, 256),
        "high" => (0.90, 512),
        _ => (0.33, 64),
    };
    (
        (max_tokens as f64 * 0.10) as i64,
        (max_tokens as f64 * max_pct) as i64,
        max_thoughts,
        switches,
    )
}

pub async fn thinkdeeper_decode(backend: &Backend<'_>, req: &Request) -> Result<ChatCompletion> {
    let max_tokens = req.opt_i64("max_tokens").unwrap_or(4096);
    let (min_d, max_d, thoughts_d, switches_d) =
        effort_profile(&req.str("reasoning_effort", "low"), max_tokens);
    let min_thinking = req.opt_i64("min_thinking_tokens").unwrap_or(min_d);
    let max_thinking = req.opt_i64("max_thinking_tokens").unwrap_or(max_d);
    let max_thoughts = req.opt_i64("max_thoughts").unwrap_or(thoughts_d);
    let switches: Vec<String> = match req.params.get("thought_switch_tokens") {
        Some(Value::Array(a)) if !a.is_empty() => a
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => switches_d,
    };
    let prefill = req.str("prefill", "");
    let start = req.str("start_think_token", "<think>");
    let end = req.str("end_think_token", "</think>");

    let head = format!("{}\n{}", start, prefill);
    let mut response = String::new();
    let mut n_thinking: i64 = 0;
    let mut thought_count: i64 = 0;
    let mut prompt_tokens = 0;
    let mut total_tokens = 0;

    // Thinking phase.
    loop {
        if n_thinking >= max_thinking || thought_count >= max_thoughts {
            break;
        }
        let budget = (max_thinking - n_thinking).max(1);
        let resp = backend
            .complete(
                &req.messages,
                &format!("{}{}", head, response),
                json!({"max_tokens": budget, "temperature": 1.0, "stop": [end]}),
            )
            .await?;
        if prompt_tokens == 0 {
            prompt_tokens = usage_prompt_tokens(&resp);
        }
        let chunk = resp.content(0).unwrap_or_default();
        let used = usage_completion_tokens(&resp);
        total_tokens += used;
        n_thinking += used;
        thought_count += switches
            .iter()
            .map(|s| chunk.matches(s.as_str()).count() as i64)
            .sum::<i64>();
        response.push_str(&chunk);
        let stopped_at_end = resp.finish_reason(0).as_deref() == Some("stop");
        if stopped_at_end && n_thinking < min_thinking {
            let replacement = {
                let mut rng = backend.rng.lock().unwrap();
                let i = rng.randbelow(switches.len().max(1));
                switches.get(i).cloned().unwrap_or_default()
            };
            response.push_str(&replacement);
            n_thinking += 1;
            thought_count += 1;
            continue;
        }
        break;
    }

    // Conclusion after the (natural or forced) end of thinking.
    response.push_str(&end);
    let resp = backend
        .complete(
            &req.messages,
            &format!("{}{}", head, response),
            json!({"max_tokens": max_tokens.max(1), "temperature": 1.0}),
        )
        .await?;
    total_tokens += usage_completion_tokens(&resp);
    response.push_str(&resp.content(0).unwrap_or_default());

    let full = format!("{}{}", head, response);
    Ok(build_completion(
        backend.model,
        &full,
        None,
        prompt_tokens,
        total_tokens,
    ))
}
