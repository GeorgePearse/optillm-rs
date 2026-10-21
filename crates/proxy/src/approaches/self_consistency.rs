//! Port of upstream `optillm/self_consistency.py`.

use serde_json::{json, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py::difflib;

const NUM_SAMPLES: usize = 5;
const SIMILARITY_THRESHOLD: f64 = 0.8;

/// `AdvancedSelfConsistency.cluster_similar_responses`.
pub fn cluster_similar_responses(responses: &[String], threshold: f64) -> Vec<Vec<String>> {
    let mut clusters: Vec<Vec<String>> = Vec::new();
    for response in responses {
        let mut added = false;
        for cluster in clusters.iter_mut() {
            if difflib::ratio(response, &cluster[0]) >= threshold {
                cluster.push(response.clone());
                added = true;
                break;
            }
        }
        if !added {
            clusters.push(vec![response.clone()]);
        }
    }
    clusters
}

pub async fn advanced_self_consistency_approach(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let max_tokens = ctx.max_tokens(4096);
    let mut tokens: i64 = 0;
    let mut responses: Vec<String> = Vec::new();
    for _ in 0..NUM_SAMPLES {
        let response = ctx
            .create(json!({
                "model": ctx.model,
                "messages": [msg("system", system_prompt), msg("user", initial_query)],
                "temperature": 1,
                "max_tokens": max_tokens
            }))
            .await?;
        if response.is_unusable() {
            tracing::warn!("Self-consistency sample was empty, None, or truncated, skipping");
            continue;
        }
        tokens += response.completion_tokens()?;
        responses.push(response.content(0).unwrap_or_default());
    }

    let mut clusters = cluster_similar_responses(&responses, SIMILARITY_THRESHOLD);
    // list.sort(key=frequency, reverse=True) is stable.
    clusters.sort_by_key(|c| std::cmp::Reverse(c.len()));

    match clusters.first() {
        Some(c) => Ok((Value::String(c[0].clone()), tokens)),
        None => Ok((json!("No consistent answer found."), tokens)),
    }
}
