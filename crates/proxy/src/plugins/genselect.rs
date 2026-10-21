//! Port of upstream `optillm/plugins/genselect_plugin.py`.

use anyhow::Result;
use regex::Regex;
use serde_json::{json, Value};

use crate::approaches::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

fn create_comparison_prompt(candidates: &[Value], query: &str) -> String {
    #[rustfmt::skip]
    let mut prompt = format!(
        concat!(
            "You are an expert evaluator tasked with selecting the best response to the following query:\n",
            "\n",
            "Query: {p0}\n",
            "\n",
            "I will provide you with {p1} different candidate responses. Please analyze each one carefully and select the best response based on the following criteria:\n",
            "\n",
            "1. **Correctness and Accuracy**: Is the response factually correct and accurate?\n",
            "2. **Completeness**: Does it fully address all aspects of the query?\n",
            "3. **Clarity**: Is the explanation clear and easy to understand?\n",
            "4. **Logical Coherence**: Is the reasoning sound and well-structured?\n",
            "5. **Practical Value**: Does it provide useful, actionable information?\n",
            "\n",
            "For coding problems, also consider:\n",
            "- Code correctness and efficiency\n",
            "- Best practices and style\n",
            "- Error handling\n",
            "\n",
            "Here are the candidate responses:\n",
            "\n",
        ),
        p0 = query,
        p1 = candidates.len(),
    );
    for (i, c) in candidates.iter().enumerate() {
        prompt += &format!("=== Candidate {} ===\n{}\n\n", i + 1, py::json::to_str(c));
    }
    #[rustfmt::skip]
    let tail = concat!(
        "Please analyze all candidates and provide:\n",
        "1. A brief comparison highlighting the strengths and weaknesses of each candidate\n",
        "2. Your selection of the best candidate (specify the number)\n",
        "3. A clear explanation of why you selected that candidate\n",
        "\n",
        "Format your response as:\n",
        "COMPARISON:\n",
        "[Your comparison analysis]\n",
        "\n",
        "BEST CANDIDATE: [number]\n",
        "\n",
        "REASONING:\n",
        "[Your explanation for the selection]",
    );
    prompt + tail
}

/// `parse_selection_response` -> (index, reasoning).
fn parse_selection(response: &str, n: usize) -> (usize, String) {
    if let Some(c) = Regex::new(r"(?i)BEST CANDIDATE:\s*(\d+)")
        .unwrap()
        .captures(response)
    {
        if let Ok(num) = c[1].parse::<i64>() {
            let idx = num - 1;
            if idx >= 0 && (idx as usize) < n {
                let reasoning = Regex::new(r"(?is)REASONING:\s*(.+)")
                    .unwrap()
                    .captures(response)
                    .map(|r| py::strings::strip(&r[1]).to_string())
                    .unwrap_or_else(|| "No explicit reasoning provided".into());
                return (idx as usize, reasoning);
            }
        }
    }
    let patterns = [
        r"[Cc]andidate\s+(\d+)\s+is\s+(?:the\s+)?best",
        r"[Ii]\s+(?:would\s+)?select\s+[Cc]andidate\s+(\d+)",
        r"[Tt]he\s+best\s+(?:response|candidate)\s+is\s+(?:number\s+)?(\d+)",
    ];
    for p in patterns {
        if let Some(c) = Regex::new(p).unwrap().captures(response) {
            if let Ok(num) = c[1].parse::<i64>() {
                let idx = num - 1;
                if idx >= 0 && (idx as usize) < n {
                    return (
                        idx as usize,
                        "Selection extracted from response pattern".into(),
                    );
                }
            }
        }
    }
    (
        0,
        "Failed to parse selection, defaulted to first candidate".into(),
    )
}

pub async fn run(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let rc = &ctx.request_config;
    let num_candidates = std::cmp::max(
        2,
        rc.get("num_candidates")
            .and_then(|v| v.as_i64())
            .unwrap_or(4),
    );
    let temperature = rc.get("temperature").cloned().unwrap_or(json!(0.7));
    let comparison_temperature = rc
        .get("comparison_temperature")
        .cloned()
        .unwrap_or(json!(0.3));
    let include_reasoning = rc
        .get("include_reasoning")
        .map(|v| v.as_bool().unwrap_or(!v.is_null()))
        .unwrap_or(false);
    let max_tokens = rc.get("max_tokens").cloned().unwrap_or(json!(4096));
    let messages = json!([msg("system", system_prompt), msg("user", initial_query)]);

    let mut candidates: Vec<Value> = Vec::new();
    let mut total_tokens: i64 = 0;
    let first: Result<()> = async {
        let response = ctx
            .create(json!({"model": ctx.model, "messages": messages, "n": num_candidates, "temperature": temperature, "max_tokens": max_tokens}))
            .await?;
        candidates = response
            .choices()
            .iter()
            .map(|c| c.get("message").and_then(|m| m.get("content")).cloned().unwrap_or(Value::Null))
            .collect();
        total_tokens += response.completion_tokens()?;
        Ok(())
    }
    .await;
    if first.is_err() {
        for _ in 0..num_candidates {
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
    if candidates.len() < 2 {
        return Ok(match candidates.first() {
            Some(c) => (c.clone(), total_tokens),
            None => (
                json!("Error: Could not generate sufficient candidates for selection"),
                total_tokens,
            ),
        });
    }

    let prompt = create_comparison_prompt(&candidates, initial_query);
    let compared: Result<Value> = async {
        let response = ctx
            .create(json!({
                "model": ctx.model,
                "messages": [
                    msg("system", "You are an expert evaluator skilled at comparing and selecting high-quality responses."),
                    msg("user", &prompt)
                ],
                "temperature": comparison_temperature,
                "max_tokens": 2048
            }))
            .await?;
        let selection = response
            .content_at(0)?
            .ok_or_else(|| anyhow::anyhow!("expected string or bytes-like object, got 'NoneType'"));
        total_tokens += response.completion_tokens()?;
        let (idx, reasoning) = parse_selection(&selection?, candidates.len());
        let selected = candidates[idx].clone();
        Ok(if include_reasoning {
            json!(format!("{}\n\n---\n**GenSelect Reasoning**: {}", py::json::to_str(&selected), reasoning))
        } else {
            selected
        })
    }
    .await;
    match compared {
        Ok(v) => Ok((v, total_tokens)),
        Err(_) => Ok((candidates[0].clone(), total_tokens)),
    }
}
