//! Port of upstream `optillm/plugins/executecode_plugin.py`. Code runs in a
//! Jupyter kernel through upstream's own `execute_code` (see `py_helper.py`),
//! whenever the plugin is selected, exactly as upstream does.

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::json;

use super::pyhelper;
use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

#[rustfmt::skip]
const EXECUTE_CODE_PROMPT: &str = concat!(
    "Generate Python code to solve this problem. Put the code in a ```python block. The code:\n",
    "1. Should use standard Python libraries (math, itertools, etc.)\n",
    "2. Should print the final answer\n",
    "3. Should be complete and runnable\n",
    "4. Should include example test cases if relevant\n",
    "\n",
    "The code will be automatically executed when submitted.",
);

fn extract_python_code(text: &str) -> Vec<String> {
    Regex::new(r"(?s)```python\s*(.*?)\s*```")
        .unwrap()
        .captures_iter(text)
        .map(|c| c[1].to_string())
        .collect()
}

async fn execute_code(code: &str) -> Result<String> {
    let r = pyhelper::call(json!({"op": "executecode", "code": code}), 120).await?;
    match r.get("ok").and_then(|v| v.as_str()) {
        Some(out) => Ok(out.to_string()),
        None => Err(anyhow!(r
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("execution failed")
            .to_string())),
    }
}

fn should_execute_request_code(query: &str) -> bool {
    let q = query.to_lowercase();
    ["run", "execute", "output", "result"]
        .iter()
        .any(|k| q.contains(k))
}

fn content_stripped(r: &crate::client::ChatCompletion) -> Result<String> {
    let c = r
        .content_at(0)?
        .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
    Ok(py::strings::strip(&c).to_string())
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    // `query, request_code = extract_python_code(initial_query)[0]` unpacks the
    // first code block (a string) into two names.
    let blocks = extract_python_code(initial_query);
    let (query, request_code) = match blocks.first() {
        Some(code) => {
            let chars: Vec<char> = code.chars().collect();
            match chars.len() {
                2 => (chars[0].to_string(), chars[1].to_string()),
                n if n > 2 => return Err(anyhow!("too many values to unpack (expected 2)")),
                n => {
                    return Err(anyhow!(
                        "not enough values to unpack (expected 2, got {})",
                        n
                    ))
                }
            }
        }
        None => (initial_query.to_string(), String::new()),
    };

    if should_execute_request_code(&query) && !request_code.is_empty() {
        let code_output = execute_code(&request_code).await?;
        let context = format!(
            "Query: {}\nCode:\n```python\n{}\n```\nOutput:\n{}",
            query, request_code, code_output
        );
        let r = ctx
            .create(json!({"model": ctx.model, "messages": [msg("system", system_prompt), msg("user", &context)]}))
            .await?;
        let text = content_stripped(&r)?;
        return Ok((json!(text), r.completion_tokens()?));
    }

    let mut messages = vec![
        msg(
            "system",
            &format!("{}{}", system_prompt, EXECUTE_CODE_PROMPT),
        ),
        msg("user", initial_query),
    ];
    let r = ctx
        .create(json!({"model": ctx.model, "messages": messages}))
        .await?;
    let initial_response = content_stripped(&r)?;
    let response_code = extract_python_code(&initial_response);
    if let Some(code) = response_code.first() {
        let code_output = execute_code(code).await?;
        let context = format!(
            "Initial response:\n{}\n\nCode output:\n{}",
            initial_response, code_output
        );
        messages.push(msg("assistant", &initial_response));
        messages.push(msg(
            "user",
            &format!(
                "Based on the code execution output, please provide a final response:\n{}",
                context
            ),
        ));
        let f = ctx
            .create(json!({"model": ctx.model, "messages": messages}))
            .await?;
        let text = content_stripped(&f)?;
        return Ok((json!(text), r.completion_tokens()? + f.completion_tokens()?));
    }
    Ok((json!(initial_response), r.completion_tokens()?))
}
