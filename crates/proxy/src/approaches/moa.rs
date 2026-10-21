//! Port of upstream `optillm/moa.py` (Mixture of Agents).

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;

pub async fn mixture_of_agents(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let mut moa_completion_tokens: i64 = 0;
    let max_tokens = ctx.max_tokens(4096);
    let mut completions: Vec<String> = Vec::new();

    let attempt: Result<()> = async {
        let provider_request = json!({
            "model": ctx.model,
            "messages": [msg("system", system_prompt), msg("user", initial_query)],
            "max_tokens": max_tokens,
            "n": 3,
            "temperature": 1
        });
        let response = ctx.create(provider_request).await?;
        if response.choices().is_empty() {
            return Err(anyhow!("Response is None or has no choices"));
        }
        completions = (0..response.choices().len())
            .filter_map(|i| response.content(i))
            .collect();
        moa_completion_tokens += response.completion_tokens()?;
        if completions.is_empty() {
            return Err(anyhow!("No valid completions generated (all were None)"));
        }
        Ok(())
    }
    .await;

    if let Err(e) = attempt {
        tracing::warn!("n parameter not supported by provider: {}", e);
        completions = Vec::new();
        for i in 0..3 {
            let provider_request = json!({
                "model": ctx.model,
                "messages": [msg("system", system_prompt), msg("user", initial_query)],
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
                tracing::warn!("Completion {}/3 truncated or empty, skipping", i + 1);
                continue;
            }
            completions.push(response.content(0).unwrap_or_default());
            match response.completion_tokens() {
                Ok(t) => moa_completion_tokens += t,
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
            moa_completion_tokens,
        ));
    }

    // Pad with the first completion to ensure we have 3.
    while completions.len() < 3 {
        completions.push(completions[0].clone());
    }

    let critique_prompt = format!(
        "
    Original query: {initial_query}

    I will present you with three candidate responses to the original query. Please analyze and critique each response, discussing their strengths and weaknesses. Provide your analysis for each candidate separately.

    Candidate 1:
    {c0}

    Candidate 2:
    {c1}

    Candidate 3:
    {c2}

    Please provide your critique for each candidate:
    ",
        initial_query = initial_query,
        c0 = completions[0],
        c1 = completions[1],
        c2 = completions[2],
    );

    let provider_request = json!({
        "model": ctx.model,
        "messages": [msg("system", system_prompt), msg("user", &critique_prompt)],
        "max_tokens": 512,
        "n": 1,
        "temperature": 0.1
    });
    let critique_response = ctx.create(provider_request).await?;
    let critiques = if critique_response.is_unusable() {
        "All candidates show reasonable approaches to the problem.".to_string()
    } else {
        critique_response.content(0).unwrap_or_default()
    };
    moa_completion_tokens += critique_response.completion_tokens()?;

    let final_prompt = format!(
        "
    Original query: {initial_query}

    Based on the following candidate responses and their critiques, generate a final response to the original query.

    Candidate 1:
    {c0}

    Candidate 2:
    {c1}

    Candidate 3:
    {c2}

    Critiques of all candidates:
    {critiques}

    Please provide a final, optimized response to the original query:
    ",
        initial_query = initial_query,
        c0 = completions[0],
        c1 = completions[1],
        c2 = completions[2],
        critiques = critiques,
    );

    let provider_request = json!({
        "model": ctx.model,
        "messages": [msg("system", system_prompt), msg("user", &final_prompt)],
        "max_tokens": max_tokens,
        "n": 1,
        "temperature": 0.1
    });
    let final_response = ctx.create(provider_request).await?;
    moa_completion_tokens += final_response.completion_tokens()?;

    let result = if final_response.is_unusable() {
        completions[0].clone()
    } else {
        final_response.content(0).unwrap_or_default()
    };
    Ok((Value::String(result), moa_completion_tokens))
}
