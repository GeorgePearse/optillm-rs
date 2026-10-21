//! Port of upstream `optillm/rto.py` (Round Trip Optimization).

use anyhow::anyhow;
use regex::Regex;
use serde_json::{json, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

/// `extract_code_from_prompt`.
pub fn extract_code_from_prompt(text: &str) -> String {
    let re = Regex::new(r"(?s)```(?:[\w-]+)?\n(.*?)```").unwrap();
    match re.captures(text) {
        Some(c) => py::strings::strip(c.get(1).unwrap().as_str()).to_string(),
        None => {
            tracing::warn!("Could not extract code from prompt. Returning original text.");
            text.to_string()
        }
    }
}

pub async fn round_trip_optimization(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let mut rto_completion_tokens: i64 = 0;
    let max_tokens = ctx.max_tokens(4096);

    let mut messages = vec![msg("system", system_prompt), msg("user", initial_query)];

    // Generate initial code (C1)
    let response_c1 = ctx
        .create(json!({
            "model": ctx.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "n": 1,
            "temperature": 0.1
        }))
        .await?;
    if response_c1.is_unusable() {
        return Err(anyhow!("RTO: provider returned an empty, None, or truncated response for the initial code (C1)"));
    }
    let c1 = response_c1.content(0).unwrap_or_default();
    rto_completion_tokens += response_c1.completion_tokens()?;

    // Generate description of the code (Q2)
    messages.push(msg("assistant", &c1));
    messages.push(msg("user", "Summarize or describe the code you just created. The summary should be in form of an instruction such that, given the instruction you can create the code yourself."));
    let response_q2 = ctx
        .create(json!({
            "model": ctx.model,
            "messages": messages,
            "max_tokens": 1024,
            "n": 1,
            "temperature": 0.1
        }))
        .await?;
    if response_q2.is_unusable() {
        return Err(anyhow!(
            "RTO: provider returned an empty, None, or truncated response for the description (Q2)"
        ));
    }
    let q2 = response_q2.content(0).unwrap_or_default();
    rto_completion_tokens += response_q2.completion_tokens()?;

    // Generate second code based on the description (C2)
    let messages = vec![msg("system", system_prompt), msg("user", &q2)];
    let response_c2 = ctx
        .create(json!({
            "model": ctx.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "n": 1,
            "temperature": 0.1
        }))
        .await?;
    if response_c2.is_unusable() {
        return Err(anyhow!(
            "RTO: provider returned an empty, None, or truncated response for the second code (C2)"
        ));
    }
    let c2 = response_c2.content(0).unwrap_or_default();
    rto_completion_tokens += response_c2.completion_tokens()?;

    let c1 = extract_code_from_prompt(&c1);
    let c2 = extract_code_from_prompt(&c2);

    if py::strings::strip(&c1) == py::strings::strip(&c2) {
        return Ok((Value::String(c1), rto_completion_tokens));
    }

    let messages = vec![
        msg("system", system_prompt),
        msg("user", &format!("Initial query: {}\n\nFirst generated code (C1):\n{}\n\nSecond generated code (C2):\n{}\n\nBased on the initial query and these two different code implementations, generate a final, optimized version of the code. Only respond with the final code, do not return anything else.", initial_query, c1, c2)),
    ];
    let response_c3 = ctx
        .create(json!({
            "model": ctx.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "n": 1,
            "temperature": 0.1
        }))
        .await?;
    if response_c3.is_unusable() {
        return Err(anyhow!(
            "RTO: provider returned an empty, None, or truncated response for the final code (C3)"
        ));
    }
    let c3 = response_c3.content(0).unwrap_or_default();
    rto_completion_tokens += response_c3.completion_tokens()?;

    Ok((Value::String(c3), rto_completion_tokens))
}
