//! Port of upstream `optillm/mars/aggregator.py` (RSA-style aggregation).

use std::sync::Mutex;

use anyhow::Result;
use serde_json::json;

use super::prompts::{MULTI_AGGREGATION_PROMPT, SINGLE_REFINEMENT_PROMPT};
use super::workspace::{AgentSolution, Workspace};
use super::{reasoning_and_total, Executor};
use crate::approaches::Ctx;
use crate::parse::msg;
use crate::py;

const SYSTEM: &str =
    "You are a mathematical reasoning expert focused on solution aggregation and refinement.";

/// Stable sort by verification score, descending (`sorted(..., reverse=True)`).
fn by_score_desc(solutions: &[AgentSolution]) -> Vec<AgentSolution> {
    let mut v = solutions.to_vec();
    v.sort_by(|a, b| {
        b.verification_score
            .partial_cmp(&a.verification_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v
}

fn sample_groups(
    ctx: &Ctx,
    ws: &Workspace,
    population_size: i64,
    aggregation_size: i64,
) -> Vec<Vec<AgentSolution>> {
    let all = &ws.solutions;
    let k = aggregation_size.max(0) as usize;
    if (all.len() as i64) < aggregation_size {
        return vec![all.clone()];
    }
    let num_samples = std::cmp::min(population_size.div_euclid(aggregation_size.max(1)), 3);
    let mut samples = Vec::new();
    for i in 0..num_samples.max(0) {
        let sample = if i == 0 {
            by_score_desc(all).into_iter().take(k).collect()
        } else if i == 1 {
            // Upstream's per-agent loop only appends when `sample` is already
            // non-empty, so it never adds anything; the sample is filled from
            // the best-by-score remainder (all solutions).
            by_score_desc(all).into_iter().take(k).collect()
        } else {
            let take = std::cmp::min(k, all.len());
            ctx.rng().sample(all, take)
        };
        samples.push(sample);
    }
    samples
}

/// `_call_model_for_aggregation`: errors yield ("", 0).
async fn call_model(
    ctx: &Ctx,
    prompt: &str,
    max_tokens: &serde_json::Value,
) -> (String, Option<i64>) {
    let result = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", SYSTEM), msg("user", prompt)],
            "max_tokens": max_tokens,
            "temperature": 0.7,
            "extra_body": {"reasoning": {"effort": "high"}}
        }))
        .await;
    let response = match result {
        Ok(r) => r,
        Err(_) => return (String::new(), Some(0)),
    };
    match response.content_at(0) {
        Ok(Some(c)) => {
            let (reasoning, _) = reasoning_and_total(&response);
            (py::strings::strip(&c).to_string(), reasoning)
        }
        _ => (String::new(), Some(0)),
    }
}

async fn aggregate_group(
    ctx: &Ctx,
    problem: &str,
    group: &[AgentSolution],
    max_tokens: &serde_json::Value,
    executor: &Executor,
) -> (Option<AgentSolution>, Option<i64>) {
    let prompt = if group.len() == 1 {
        py::strings::format_named(
            SINGLE_REFINEMENT_PROMPT,
            &[
                ("problem", problem),
                ("candidate_solution", &group[0].solution),
            ],
        )
    } else {
        let mut candidate_text = String::new();
        for (i, sol) in group.iter().enumerate() {
            candidate_text += &format!(
                "Solution {} (Agent {}, confidence: {:.2}):\n",
                i + 1,
                sol.agent_id,
                sol.confidence
            );
            candidate_text += &sol.solution;
            candidate_text += "\n\n";
        }
        py::strings::format_named(
            MULTI_AGGREGATION_PROMPT,
            &[
                ("problem", problem),
                ("candidate_solutions", &candidate_text),
            ],
        )
    };
    let Ok(prompt) = prompt else {
        return (None, Some(0));
    };
    let (solution, tokens) = executor.run(call_model(ctx, &prompt, max_tokens)).await;
    if solution.is_empty() {
        return (None, tokens);
    }
    let agent_id = format!("agg_{}", chrono::Local::now().format("%H%M%S"));
    (
        Some(AgentSolution {
            agent_id,
            solution,
            confidence: 0.8,
            reasoning_tokens: tokens,
            is_verified: false,
            verification_score: 0.0,
            temperature: 0.7,
            verification_results: Vec::new(),
        }),
        tokens,
    )
}

/// `run_aggregation_loops` -> total reasoning tokens.
pub async fn run_aggregation_loops(
    ctx: &Ctx,
    ws: &Mutex<Workspace>,
    executor: &Executor,
) -> Result<Option<i64>> {
    let (population_size, aggregation_size, loops, max_tokens) = {
        let w = ws.lock().unwrap();
        (
            w.config.int("population_size", 6),
            w.config.int("aggregation_size", 3),
            w.config.int("aggregation_loops", 3),
            w.config.get_or("max_tokens", json!(30000)),
        )
    };
    let mut total: Option<i64> = Some(0);
    for _ in 0..loops.max(0) {
        let (problem, groups) = {
            let w = ws.lock().unwrap();
            (
                w.problem.clone(),
                sample_groups(ctx, &w, population_size, aggregation_size),
            )
        };
        let results = futures::future::join_all(
            groups
                .iter()
                .map(|g| aggregate_group(ctx, &problem, g, &max_tokens, executor)),
        )
        .await;
        let mut new_solutions = Vec::new();
        let mut loop_tokens: Option<i64> = Some(0);
        for (solution, tokens) in results {
            if let Some(s) = solution {
                new_solutions.push(s);
            }
            loop_tokens = match (loop_tokens, tokens) {
                (Some(a), Some(b)) => Some(a + b),
                _ => {
                    return Err(anyhow::anyhow!(
                        "unsupported operand type(s) for +=: 'int' and 'NoneType'"
                    ))
                }
            };
        }
        // _update_population
        {
            let mut w = ws.lock().unwrap();
            for s in new_solutions {
                w.add_solution(s)?;
            }
            if w.solutions.len() as i64 > population_size {
                let mut sorted = w.solutions.clone();
                sorted.sort_by(|a, b| {
                    (b.verification_score, b.confidence)
                        .partial_cmp(&(a.verification_score, a.confidence))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted.truncate(population_size.max(0) as usize);
                w.solutions = sorted;
            }
        }
        total = match (total, loop_tokens) {
            (Some(a), Some(b)) => Some(a + b),
            _ => {
                return Err(anyhow::anyhow!(
                    "unsupported operand type(s) for +=: 'int' and 'NoneType'"
                ))
            }
        };
    }
    Ok(total)
}
