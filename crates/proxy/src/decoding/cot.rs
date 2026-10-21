//! `cot_decoding` (Wang & Zhou, 2024) over API logprobs; mirrors
//! upstream `optillm/cot_decoding.py`.

use anyhow::{anyhow, Result};
use serde_json::json;

use super::{
    build_completion, token_logprobs, usage_completion_tokens, usage_prompt_tokens, Backend,
    Request,
};
use crate::client::ChatCompletion;

/// `calculate_confidence`: mean over generated steps of p(top1) - p(top2).
fn calculate_confidence(steps: &[(String, f64, Vec<f64>)]) -> f64 {
    if steps.is_empty() {
        return 0.0;
    }
    let sum: f64 = steps
        .iter()
        .map(|(_, _, tops)| {
            let mut probs: Vec<f64> = tops.iter().map(|lp| lp.exp()).collect();
            probs.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            if probs.len() > 1 {
                probs[0] - probs[1]
            } else {
                1.0
            }
        })
        .sum();
    sum / steps.len() as f64
}

pub async fn cot_decode(backend: &Backend<'_>, req: &Request) -> Result<ChatCompletion> {
    let k = req.i64("k", 10).clamp(1, 20);
    let max_new_tokens = req.opt_i64("max_tokens").unwrap_or(512);
    let aggregate = req.bool("aggregate_paths", true);

    // First step: the top-k candidate first tokens.
    let first = backend
        .complete(
            &req.messages,
            "",
            json!({"max_tokens": 1, "temperature": 0, "logprobs": true, "top_logprobs": k}),
        )
        .await?;
    let candidates: Vec<String> = first
        .choices()
        .first()
        .and_then(|c| c.get("logprobs"))
        .and_then(|l| l.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|t| t.get("top_logprobs"))
        .and_then(|t| t.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    x.get("token")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .ok_or_else(|| anyhow!("cot_decoding needs a backend that returns top_logprobs"))?;
    let prompt_tokens = usage_prompt_tokens(&first);

    // Greedy continuation of each candidate.
    let mut paths: Vec<(String, f64)> = Vec::new();
    let mut tokens_by_answer: Vec<i64> = Vec::new();
    for token in &candidates {
        let resp = backend
            .complete(
                &req.messages,
                token,
                json!({"max_tokens": max_new_tokens, "temperature": 0, "logprobs": true, "top_logprobs": 2}),
            )
            .await?;
        let steps = token_logprobs(&resp);
        let answer = format!("{}{}", token, resp.content(0).unwrap_or_default());
        paths.push((answer, calculate_confidence(&steps)));
        tokens_by_answer.push(usage_completion_tokens(&resp) + 1);
    }
    if paths.is_empty() {
        return Err(anyhow!("cot_decoding got no candidate first tokens"));
    }

    let (answer, confidence, idx) = if aggregate {
        // aggregate_paths_based_on_scores: sum deltas per identical answer.
        let mut scores: Vec<(String, f64, usize)> = Vec::new();
        for (i, (a, d)) in paths.iter().enumerate() {
            match scores.iter_mut().find(|(x, _, _)| x == a) {
                Some(e) => e.1 += d,
                None => scores.push((a.clone(), *d, i)),
            }
        }
        let best = crate::py::strings::argmax_by(&scores, |s| s.1).unwrap_or(0);
        scores.swap_remove(best)
    } else {
        let best = crate::py::strings::argmax_by(&paths, |p| p.1).unwrap_or(0);
        (paths[best].0.clone(), paths[best].1, best)
    };
    Ok(build_completion(
        backend.model,
        &answer,
        Some(json!({"confidence_score": confidence})),
        prompt_tokens,
        tokens_by_answer[idx],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_is_mean_gap_between_top_two() {
        let steps = vec![
            ("a".to_string(), 0.0, vec![0.9f64.ln(), 0.1f64.ln()]),
            ("b".to_string(), 0.0, vec![0.5f64.ln(), 0.3f64.ln()]),
        ];
        let c = calculate_confidence(&steps);
        assert!((c - 0.5).abs() < 1e-9);
        assert_eq!(calculate_confidence(&[("x".into(), 0.0, vec![0.0])]), 1.0);
    }
}
