//! Port of upstream `optillm/mars` (MARS: multi-agent reasoning system).
//!
//! Upstream runs every phase through one `ThreadPoolExecutor` of
//! `max(num_agents, num_agents * min(2, verification_passes_required))`
//! workers under `asyncio.gather`; here each executor job holds a permit of a
//! FIFO semaphore of the same size, so the same requests are in flight at the
//! same time and concurrent fan-out resumes in the same order under the
//! harness's serialised mock.
//!
//! Quirks preserved on purpose: solutions carry string agent ids while agents
//! carry int ids, so the first agent verifies everything and the iterative
//! improvement step never finds an agent; aggregated solutions are named
//! `agg_HHMMSS` from the local wall clock.

mod agent;
mod aggregator;
mod extraction;
mod prompts;
mod strategy;
mod verifier;
mod workspace;

use std::sync::Mutex;

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use tokio::sync::Semaphore;

use super::{ApproachResult, Ctx};
use crate::client::ChatCompletion;
use crate::parse::msg;
use crate::py;
use agent::MarsAgent;
use workspace::Workspace;

/// A MARS config dict with Python `dict.get(key, default)` semantics.
#[derive(Clone, Debug)]
pub struct Config(Map<String, Value>);

impl Config {
    fn default_config() -> Self {
        Config(
            json!({
                "num_agents": 3,
                "max_iterations": 5,
                "verification_passes_required": 2,
                "consensus_threshold": 2,
                "min_verified_solutions": 1,
                "max_tokens": 64000,
                "max_verification_attempts": 3,
                "early_termination": true,
                "use_reasoning_api": true,
                "enable_aggregation": true,
                "population_size": 6,
                "aggregation_size": 3,
                "aggregation_loops": 3,
                "enable_strategy_network": true,
                "strategy_extraction_enabled": true,
                "cross_agent_enhancement": true,
                "use_thinking_tags": true,
                "answer_extraction_mode": "auto"
            })
            .as_object()
            .unwrap()
            .clone(),
        )
    }

    fn lightweight_config() -> Self {
        Config(
            json!({
                "num_agents": 2,
                "max_iterations": 2,
                "verification_passes_required": 1,
                "consensus_threshold": 1,
                "min_verified_solutions": 1,
                "max_tokens": 4000,
                "max_verification_attempts": 2,
                "early_termination": true,
                "use_reasoning_api": true,
                "enable_aggregation": false,
                "enable_strategy_network": false,
                "strategy_extraction_enabled": false,
                "cross_agent_enhancement": false,
                "use_thinking_tags": true,
                "answer_extraction_mode": "auto"
            })
            .as_object()
            .unwrap()
            .clone(),
        )
    }

    pub fn get(&self, k: &str) -> Value {
        self.0.get(k).cloned().unwrap_or(Value::Null)
    }

    pub fn get_or(&self, k: &str, default: Value) -> Value {
        self.0.get(k).cloned().unwrap_or(default)
    }

    pub fn int(&self, k: &str, default: i64) -> i64 {
        match self.0.get(k) {
            Some(v) => v
                .as_i64()
                .or_else(|| v.as_f64().map(|f| f as i64))
                .unwrap_or(default),
            None => default,
        }
    }

    pub fn truthy(&self, k: &str, default: bool) -> bool {
        match self.0.get(k) {
            None => default,
            Some(Value::Bool(b)) => *b,
            Some(Value::Null) => false,
            Some(Value::Number(n)) => n.as_f64() != Some(0.0),
            Some(Value::String(s)) => !s.is_empty(),
            Some(Value::Array(a)) => !a.is_empty(),
            Some(Value::Object(o)) => !o.is_empty(),
        }
    }
}

/// The shared `ThreadPoolExecutor`.
pub struct Executor(Semaphore);

impl Executor {
    pub async fn run<F: std::future::Future>(&self, job: F) -> F::Output {
        let _permit = self.0.acquire().await.expect("executor closed");
        job.await
    }
}

/// Upstream's getattr chain over `response.usage` -> (reasoning_tokens, total_tokens);
/// `None` stands for a Python `None` that leaked through.
pub fn reasoning_and_total(response: &ChatCompletion) -> (Option<i64>, Option<i64>) {
    let as_opt = |v: Option<&Value>| -> Option<i64> {
        match v {
            Some(Value::Null) => None,
            Some(v) => Some(v.as_i64().unwrap_or(0)),
            None => Some(0),
        }
    };
    let Some(Value::Object(usage)) = response.0.get("usage") else {
        return (Some(0), Some(0));
    };
    let total = as_opt(usage.get("total_tokens"));
    let mut reasoning = Some(0);
    if let Some(Value::Object(details)) = usage.get("completion_tokens_details") {
        reasoning = as_opt(details.get("reasoning_tokens"));
    }
    if reasoning == Some(0) {
        reasoning = as_opt(usage.get("reasoning_tokens"));
    }
    (reasoning, total)
}

fn add(a: Option<i64>, b: Option<i64>) -> Result<Option<i64>> {
    match (a, b) {
        (Some(x), Some(y)) => Ok(Some(x + y)),
        _ => Err(anyhow!(
            "unsupported operand type(s) for +=: 'int' and 'NoneType'"
        )),
    }
}

fn agent_solution_summary(ws: &Workspace) -> Result<()> {
    // `_log_solution_overview` formats `{reasoning_tokens:,}`, which raises for None.
    for s in &ws.solutions {
        if s.reasoning_tokens.is_none() {
            return Err(anyhow!(
                "unsupported format string passed to NoneType.__format__"
            ));
        }
    }
    Ok(())
}

/// `_synthesize_final_solution` -> (final_solution, reasoning_tokens). The
/// voting extraction runs outside upstream's try block, so its errors abort
/// the whole MARS run (and trigger the single-agent fallback).
async fn synthesize(
    ctx: &Ctx,
    ws: &Mutex<Workspace>,
    config: &Config,
) -> Result<(String, Option<i64>)> {
    let (best_verified, solutions) = {
        let w = ws.lock().unwrap();
        let best = w
            .get_best_solution()
            .filter(|b| b.is_verified)
            .map(|b| b.solution.clone());
        (best, w.solutions.clone())
    };
    if let Some(s) = best_verified {
        return Ok((s, Some(0)));
    }

    let mut entries = Vec::new();
    for s in &solutions {
        entries.push(extraction::voting_entries(&s.solution)?);
    }
    let mut numerical: Vec<(i64, usize)> = Vec::new();
    let mut extracted_info: Vec<(String, String, String)> = Vec::new();
    for (i, es) in entries.iter().enumerate() {
        for e in es {
            if e.kind == "unified_numeric" {
                if let Some(n) = e.int {
                    numerical.push((n, i));
                }
            }
            extracted_info.push((
                e.text.clone(),
                solutions[i].agent_id.clone(),
                e.kind.to_string(),
            ));
        }
    }

    if numerical.len() >= 2 {
        // Counter.most_common(): highest count first, ties in insertion order.
        let mut counts: Vec<(i64, usize)> = Vec::new();
        for (n, _) in &numerical {
            match counts.iter_mut().find(|(k, _)| k == n) {
                Some(c) => c.1 += 1,
                None => counts.push((*n, 1)),
            }
        }
        let (answer, count) = counts
            .iter()
            .fold(None::<(i64, usize)>, |acc, &(k, c)| match acc {
                Some((_, bc)) if bc >= c => acc,
                _ => Some((k, c)),
            })
            .unwrap();
        if count >= 2 {
            let matching: Vec<&workspace::AgentSolution> = numerical
                .iter()
                .filter(|(n, _)| *n == answer)
                .map(|(_, i)| &solutions[*i])
                .collect();
            let idx = py::strings::argmax_by(&matching, |s| s.confidence).unwrap_or(0);
            return Ok((matching[idx].solution.clone(), Some(0)));
        }
    }

    let (problem, summary) = {
        let w = ws.lock().unwrap();
        (w.problem.clone(), w.verification_summary())
    };
    let mut agent_solutions_text = String::new();
    for s in solutions.iter().take(3) {
        agent_solutions_text += &format!(
            "\nAgent {} (confidence: {:.2}):\n",
            s.agent_id, s.confidence
        );
        agent_solutions_text += &s.solution;
        agent_solutions_text += &format!("\n{}\n", "=".repeat(50));
    }
    let verification_text = format!("Verification Summary: {}", py::json::repr(&summary));
    let mut extracted_answers_text = String::new();
    if !extracted_info.is_empty() {
        extracted_answers_text = "\n\nEXTRACTED ANSWERS FROM AGENTS:\n".to_string();
        for (answer, agent_id, method) in &extracted_info {
            extracted_answers_text +=
                &format!("- Agent {}: '{}' (via {})\n", agent_id, answer, method);
        }
        extracted_answers_text += "\nIMPORTANT: If multiple agents extracted the same answer, prioritize it in your synthesis.\n";
        extracted_answers_text += "Ensure the final answer is clearly formatted and matches the expected answer format.\n";
    }
    let synthesis_prompt = match py::strings::format_named(
        prompts::SYNTHESIS_PROMPT,
        &[
            ("problem", &problem),
            ("agent_solutions", &agent_solutions_text),
            ("verification_results", &verification_text),
        ],
    ) {
        Ok(p) => p + &extracted_answers_text,
        Err(_) => String::new(),
    };

    let result = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", "You are a mathematical synthesis expert."), msg("user", &synthesis_prompt)],
            "max_tokens": config.get("max_tokens"),
            "temperature": 0.3,
            "extra_body": {"reasoning": {"effort": "high"}}
        }))
        .await;
    let synthesized = result.ok().and_then(|r| match r.content_at(0) {
        Ok(Some(c)) => Some((
            py::strings::strip(&c).to_string(),
            reasoning_and_total(&r).0,
        )),
        _ => None,
    });
    Ok(match synthesized {
        Some(s) => s,
        None => {
            if solutions.is_empty() {
                return Ok((
                    "Unable to generate solution due to synthesis failure.".into(),
                    Some(0),
                ));
            }
            let idx = py::strings::argmax_by(&solutions, |s| s.verification_score).unwrap_or(0);
            (solutions[idx].solution.clone(), Some(0))
        }
    })
}

async fn run_phases(
    ctx: &Ctx,
    initial_query: &str,
    config: &Config,
) -> Result<(String, Option<i64>)> {
    let num_agents = config.int("num_agents", 3);
    let passes = config.int("verification_passes_required", 2);
    let max_workers =
        std::cmp::max(num_agents, num_agents * std::cmp::min(2, passes)).max(1) as usize;
    let executor = Executor(Semaphore::new(max_workers));
    let ws = Mutex::new(Workspace::new(initial_query, config.clone()));
    let agents: Vec<MarsAgent> = (0..num_agents.max(0))
        .map(|i| MarsAgent::new(i, config))
        .collect();
    let mut total: Option<i64> = Some(0);

    // Phase 1: exploration.
    let results = futures::future::join_all(
        agents
            .iter()
            .map(|a| executor.run(a.generate_solution(ctx, initial_query))),
    )
    .await;
    let mut exploration_tokens = Some(0);
    for (solution, tokens) in results {
        ws.lock().unwrap().add_solution(solution)?;
        exploration_tokens = add(exploration_tokens, tokens)?;
    }
    total = add(total, exploration_tokens)?;

    // Phase 2a: aggregation.
    if config.truthy("enable_aggregation", true) {
        let t = aggregator::run_aggregation_loops(ctx, &ws, &executor).await?;
        total = add(total, t)?;
    }

    // Phase 2b: strategy network.
    if config.truthy("enable_strategy_network", true)
        && config.truthy("strategy_extraction_enabled", true)
    {
        let extracted = strategy::extract_strategies(ctx, &ws, &executor).await;
        if config.truthy("cross_agent_enhancement", true) && !extracted.is_empty() {
            strategy::share_strategies(ctx, &ws, &extracted, &executor).await?;
        }
    }

    // Phase 3: verification.
    verifier::verify_solutions_parallel(ctx, &agents, &ws, &executor).await;

    // Phase 4: iterative improvement.
    let max_iterations = config.int("max_iterations", 5);
    let mut iteration_count = 0;
    while ws.lock().unwrap().should_continue_iteration() && iteration_count < max_iterations {
        iteration_count += 1;
        let improved = verifier::iterative_improvement_parallel().await;
        total = add(total, Some(improved))?;
        verifier::verify_solutions_parallel(ctx, &agents, &ws, &executor).await;
        if config.truthy("early_termination", false) && ws.lock().unwrap().has_consensus() {
            break;
        }
        ws.lock().unwrap().iteration_count = iteration_count;
    }

    // Phase 5: synthesis.
    agent_solution_summary(&ws.lock().unwrap())?;
    let (final_solution, synthesis_tokens) = synthesize(ctx, &ws, config).await?;
    total = add(total, synthesis_tokens)?;

    if config.truthy("use_thinking_tags", true) {
        let mode = py::json::to_str(&config.get_or("answer_extraction_mode", json!("auto")));
        let clean = extraction::extract_clean_answer(&final_solution, &mode);
        return Ok((
            extraction::wrap_with_thinking_tags(&final_solution, &clean),
            total,
        ));
    }
    Ok((final_solution, total))
}

pub async fn multi_agent_reasoning_system(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let _ = system_prompt; // upstream accepts but never uses it
    let rc = &ctx.request_config;
    let use_lightweight = match rc.get("max_tokens") {
        None => false,
        Some(v) => match v.as_f64() {
            Some(m) => m <= 4000.0,
            None => {
                return Err(anyhow!(
                    "'<=' not supported between instances of 'NoneType' and 'int'"
                ))
            }
        },
    };
    let mut config = if use_lightweight {
        Config::lightweight_config()
    } else {
        Config::default_config()
    };
    if let Some(Value::Object(overrides)) = rc.get("mars_config") {
        for (k, v) in overrides {
            config.0.insert(k.clone(), v.clone());
        }
    }
    if let Some(mt) = rc.get("max_tokens") {
        config.0.insert("max_tokens".into(), mt.clone());
    }

    match run_phases(ctx, initial_query, &config).await {
        Ok((answer, tokens)) => Ok((Value::String(answer), tokens.unwrap_or(0))),
        Err(e) => {
            tracing::error!("MARS execution failed: {}", e);
            let fallback = MarsAgent::new(0, &config);
            let (solution, tokens) = fallback.generate_solution(ctx, initial_query).await;
            Ok((Value::String(solution.solution), tokens.unwrap_or(0)))
        }
    }
}
