//! Port of upstream `optillm/bon.py` (Best-of-N sampling).

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

const RATING_PROMPT: &str = "Rate the following responses on a scale from 0 to 10, where 0 is poor and 10 is excellent. Consider factors such as relevance, coherence, and helpfulness. Respond with only a number.";

pub async fn best_of_n_sampling(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
    n: i64,
) -> ApproachResult {
    let mut bon_completion_tokens: i64 = 0;
    let max_tokens = ctx.max_tokens(4096);
    let messages = vec![msg("system", system_prompt), msg("user", initial_query)];
    let mut completions: Vec<String> = Vec::new();

    // try: generate n completions in a single API call using the n parameter.
    let attempt: Result<()> = async {
        let provider_request = json!({
            "model": ctx.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "n": n,
            "temperature": 1
        });
        let response = ctx.create(provider_request).await?;
        if response.choices().is_empty() {
            return Err(anyhow!("Response is None or has no choices"));
        }
        completions = (0..response.choices().len())
            .filter_map(|i| response.content(i))
            .collect();
        let tokens = response.completion_tokens()?;
        bon_completion_tokens += tokens;
        if completions.is_empty() {
            return Err(anyhow!("No valid completions generated (all were None)"));
        }
        Ok(())
    }
    .await;

    if let Err(e) = attempt {
        tracing::warn!("n parameter not supported by provider: {}", e);
        tracing::info!("Falling back to generating {} completions one by one", n);
        for i in 0..n {
            let provider_request = json!({
                "model": ctx.model,
                "messages": messages,
                "max_tokens": max_tokens,
                "temperature": 1
            });
            let response = match ctx.create(provider_request).await {
                Ok(r) => r,
                Err(err) => {
                    tracing::error!("Error generating completion {}: {}", i + 1, err);
                    continue;
                }
            };
            if response.is_unusable() {
                tracing::warn!("Completion {}/{} truncated or empty, skipping", i + 1, n);
                continue;
            }
            completions.push(response.content(0).unwrap_or_default());
            match response.completion_tokens() {
                Ok(t) => bon_completion_tokens += t,
                Err(err) => {
                    tracing::error!("Error generating completion {}: {}", i + 1, err);
                    continue;
                }
            }
        }
        if completions.is_empty() {
            return Ok((json!("Error: Could not generate any completions"), 0));
        }
    }

    if completions.is_empty() {
        return Ok((
            json!("Error: Could not generate any completions"),
            bon_completion_tokens,
        ));
    }

    // Rate the completions.
    let mut rating_messages = messages.clone();
    rating_messages.push(msg("system", RATING_PROMPT));

    let mut ratings: Vec<f64> = Vec::new();
    for completion in &completions {
        rating_messages.push(msg("assistant", completion));
        rating_messages.push(msg("user", "Rate the above response:"));
        let provider_request = json!({
            "model": ctx.model,
            "messages": rating_messages,
            "max_tokens": 256,
            "n": 1,
            "temperature": 0.1
        });
        let rating_response = ctx.create(provider_request).await?;
        bon_completion_tokens += rating_response.completion_tokens()?;
        if rating_response.is_unusable() {
            ratings.push(0.0);
        } else {
            let content = rating_response.content(0).unwrap_or_default();
            ratings.push(py::strings::parse_float(py::strings::strip(&content)).unwrap_or(0.0));
        }
        rating_messages.truncate(rating_messages.len() - 2);
    }

    let best_index = py::strings::argmax_by(&ratings, |r| *r).unwrap_or(0);
    Ok((
        Value::String(completions[best_index].clone()),
        bon_completion_tokens,
    ))
}
