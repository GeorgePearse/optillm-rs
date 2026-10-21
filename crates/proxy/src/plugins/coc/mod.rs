//! Port of upstream `optillm/plugins/coc_plugin.py` (Chain of Code). Code
//! execution and `ast.literal_eval` run through upstream's own functions in
//! `py_helper.py`.

mod prompts;

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Value};

use super::pyhelper;
use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;
use prompts::{CHAIN_OF_CODE_PROMPT, CODE_FIX_PROMPT, SIMULATION_PROMPT};

const MAX_FIX_ATTEMPTS: i64 = 3;

fn extract_code_blocks(text: &str) -> Vec<String> {
    Regex::new(r"(?s)```python\s*(.*?)\s*```")
        .unwrap()
        .captures_iter(text)
        .map(|c| py::strings::strip(&c[1]).to_string())
        .collect()
}

/// `execute_code` -> (str(answer), error).
async fn execute_code(code: &str) -> (Option<String>, Option<String>) {
    match pyhelper::call(json!({"op": "coc", "code": code}), 120).await {
        Ok(r) => (
            r.get("answer")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            r.get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        ),
        Err(e) => (None, Some(format!("Notebook execution failed: {}", e))),
    }
}

fn text_of(r: &crate::client::ChatCompletion) -> Result<Value> {
    if r.choices().is_empty() {
        return Err(anyhow!("list index out of range"));
    }
    Ok(r.message(0)
        .and_then(|m| m.get("content"))
        .cloned()
        .unwrap_or(Value::Null))
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let r = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", &format!("{}\n{}", system_prompt, CHAIN_OF_CODE_PROMPT)), msg("user", initial_query)],
            "temperature": 0.7
        }))
        .await?;
    let mut total_tokens = r.completion_tokens()?;
    let first = text_of(&r)?;
    let blocks = extract_code_blocks(
        first
            .as_str()
            .ok_or_else(|| anyhow!("expected string or bytes-like object, got 'NoneType'"))?,
    );
    let Some(mut current_code) = blocks.into_iter().next() else {
        return Ok((first, total_tokens));
    };

    let mut fix_attempts = 0;
    let mut last_error = String::new();
    while fix_attempts < MAX_FIX_ATTEMPTS {
        fix_attempts += 1;
        let (answer, error) = execute_code(&current_code).await;
        match error {
            None => return Ok((json!(answer.unwrap_or_default()), total_tokens)),
            Some(e) => last_error = e,
        }
        if fix_attempts >= MAX_FIX_ATTEMPTS {
            break;
        }
        let system = py::strings::format_named(
            CODE_FIX_PROMPT,
            &[("code", &current_code), ("error", &last_error)],
        )
        .map_err(|e| anyhow!(e))?;
        let fr = ctx
            .create(json!({
                "model": ctx.model,
                "messages": [msg("system", &system), msg("user", "Fix the code to make it work.")],
                "temperature": 0.2
            }))
            .await?;
        let fixed = text_of(&fr)?;
        let fixed_blocks = extract_code_blocks(
            fixed
                .as_str()
                .ok_or_else(|| anyhow!("expected string or bytes-like object, got 'NoneType'"))?,
        );
        total_tokens += fr.completion_tokens()?;
        match fixed_blocks.into_iter().next() {
            Some(c) => current_code = c,
            None => break,
        }
    }

    let system = py::strings::format_named(
        SIMULATION_PROMPT,
        &[("code", &current_code), ("error", &last_error)],
    )
    .map_err(|e| anyhow!(e))?;
    let sr = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", &system), msg("user", "Simulate this code and return the final answer value.")],
            "temperature": 0.2
        }))
        .await?;
    let simulated = match sr.content(0) {
        Some(c) => {
            let text = py::strings::strip(&c).to_string();
            let r = pyhelper::call(json!({"op": "literal_str", "text": text}), 60).await;
            Some(
                r.ok()
                    .and_then(|v| v.get("ok").and_then(|s| s.as_str()).map(|s| s.to_string()))
                    .unwrap_or(text),
            )
        }
        None => None,
    };
    total_tokens += sr.completion_tokens()?;
    match simulated {
        Some(s) => Ok((json!(s), total_tokens)),
        None => Ok((
            json!(format!(
                "Error: Could not solve problem after all attempts. Last error: {}",
                last_error
            )),
            total_tokens,
        )),
    }
}
