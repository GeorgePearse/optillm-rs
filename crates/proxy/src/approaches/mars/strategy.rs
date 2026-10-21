//! Port of upstream `optillm/mars/strategy_network.py` (cross-agent strategy sharing).

use std::sync::Mutex;

use anyhow::Result;
use serde_json::{json, Value};

use super::workspace::{AgentSolution, Workspace};
use super::{reasoning_and_total, Executor};
use crate::approaches::{py_min, Ctx};
use crate::parse::msg;
use crate::py;

#[derive(Clone, Debug)]
pub struct Strategy {
    pub agent_id: String,
    pub problem_type: String,
    pub approach_type: String,
    pub key_insights: Vec<String>,
    pub techniques: Vec<String>,
    pub solution_pattern: String,
    pub confidence: f64,
}

#[derive(Default)]
struct Parsed {
    problem_type: Option<String>,
    approach_type: Option<String>,
    key_insights: Option<Vec<String>>,
    techniques: Option<Vec<String>>,
    solution_pattern: Option<String>,
    any: bool,
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|v| py::strings::strip(v).to_string())
        .collect()
}

/// `_parse_strategy_analysis`.
fn parse_strategy_analysis(analysis: &str) -> Option<Parsed> {
    let mut p = Parsed::default();
    for line in analysis.split('\n') {
        let line = py::strings::strip(line);
        if let Some((key, value)) = line.split_once(':') {
            let key = py::strings::strip(key).to_lowercase();
            let value = py::strings::strip(value).to_string();
            if key == "problem_type" {
                p.problem_type = Some(value);
            } else if key == "approach_type" {
                p.approach_type = Some(value);
            } else if key.contains("insights") {
                p.key_insights = Some(split_list(&value));
            } else if key.contains("techniques") {
                p.techniques = Some(split_list(&value));
            } else if key.contains("pattern") {
                p.solution_pattern = Some(value);
            } else if key.contains("indicators") {
                // success_indicators: parsed but never used downstream.
            } else {
                continue;
            }
            p.any = true;
        }
    }
    p.any.then_some(p)
}

async fn extract_strategy(
    ctx: &Ctx,
    solution: &AgentSolution,
    problem: &str,
    max_tokens: &Value,
) -> Option<Strategy> {
    #[rustfmt::skip]
    let prompt = format!(
        concat!(
            "Analyze this mathematical solution and extract the key reasoning strategy:\n",
            "\n",
            "Problem: {p0}\n",
            "\n",
            "Agent Solution:\n",
            "{p1}\n",
            "\n",
            "Extract the following strategy components:\n",
            "\n",
            "1. PROBLEM_TYPE: Classify as one of [algebra, geometry, combinatorics, number_theory, calculus, discrete_math, probability]\n",
            "\n",
            "2. APPROACH_TYPE: Identify the main approach [direct_computation, proof_by_contradiction, constructive_proof, case_analysis, induction, algebraic_manipulation, geometric_visualization, pattern_recognition, reduction_to_known_problem]\n",
            "\n",
            "3. KEY_INSIGHTS: List 2-3 key mathematical insights that enabled the solution\n",
            "\n",
            "4. MATHEMATICAL_TECHNIQUES: List specific techniques used [substitution, factorization, coordinate_geometry, symmetry, pigeonhole_principle, etc.]\n",
            "\n",
            "5. SOLUTION_PATTERN: Describe the general pattern/template of this solution approach\n",
            "\n",
            "6. SUCCESS_INDICATORS: What makes this approach particularly effective for this type of problem?\n",
            "\n",
            "Format your response as:\n",
            "PROBLEM_TYPE: [type]\n",
            "APPROACH_TYPE: [approach]\n",
            "KEY_INSIGHTS: [insight1], [insight2], [insight3]\n",
            "MATHEMATICAL_TECHNIQUES: [technique1], [technique2], [technique3]\n",
            "SOLUTION_PATTERN: [pattern description]\n",
            "SUCCESS_INDICATORS: [indicator1], [indicator2]",
        ),
        p0 = problem,
        p1 = solution.solution,
    );
    let quarter = match max_tokens.as_i64() {
        Some(m) => json!(m.div_euclid(4)),
        None => json!((max_tokens.as_f64().unwrap_or(0.0) / 4.0).floor()),
    };
    let response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [
                msg("system", "You are a mathematical strategy analysis expert. Extract reasoning patterns from solutions."),
                msg("user", &prompt)
            ],
            "max_tokens": quarter,
            "temperature": 0.3,
            "extra_body": {"reasoning": {"effort": "medium"}}
        }))
        .await
        .ok()?;
    let analysis = py::strings::strip(&response.content_at(0).ok()??).to_string();
    let parsed = parse_strategy_analysis(&analysis)?;
    Some(Strategy {
        agent_id: solution.agent_id.clone(),
        problem_type: parsed.problem_type.unwrap_or_else(|| "unknown".into()),
        approach_type: parsed.approach_type.unwrap_or_else(|| "unknown".into()),
        key_insights: parsed.key_insights.unwrap_or_default(),
        techniques: parsed.techniques.unwrap_or_default(),
        solution_pattern: parsed.solution_pattern.unwrap_or_default(),
        confidence: solution.confidence,
    })
}

/// `extract_strategies_from_solutions` (insertion-ordered list of strategies).
pub async fn extract_strategies(
    ctx: &Ctx,
    ws: &Mutex<Workspace>,
    executor: &Executor,
) -> Vec<Strategy> {
    let (problem, solutions, max_tokens) = {
        let w = ws.lock().unwrap();
        (
            w.problem.clone(),
            w.solutions.clone(),
            w.config.get_or("max_tokens", json!(30000)),
        )
    };
    let jobs = solutions
        .iter()
        .filter(|s| !s.agent_id.starts_with("agg_"))
        .map(|s| executor.run(extract_strategy(ctx, s, &problem, &max_tokens)));
    futures::future::join_all(jobs)
        .await
        .into_iter()
        .flatten()
        .collect()
}

async fn classify_problem(ctx: &Ctx, problem: &str) -> String {
    #[rustfmt::skip]
    let prompt = format!(
        concat!(
            "Classify this mathematical problem into one category:\n",
            "\n",
            "Problem: {p0}\n",
            "\n",
            "Categories: [algebra, geometry, combinatorics, number_theory, calculus, discrete_math, probability]\n",
            "\n",
            "Respond with just the category name.",
        ),
        p0 = problem,
    );
    let response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", "You are a mathematical problem classifier."), msg("user", &prompt)],
            "max_tokens": 50,
            "temperature": 0.1,
            "extra_body": {"reasoning": {"effort": "low"}}
        }))
        .await;
    let classification = match response.ok().and_then(|r| r.content_at(0).ok().flatten()) {
        Some(c) => py::strings::strip(&c).to_lowercase(),
        None => return "algebra".into(),
    };
    let valid = [
        "algebra",
        "geometry",
        "combinatorics",
        "number_theory",
        "calculus",
        "discrete_math",
        "probability",
    ];
    if valid.contains(&classification.as_str()) {
        classification
    } else {
        "algebra".into()
    }
}

fn effective_strategies(problem_type: &str, extracted: &[Strategy]) -> Vec<Strategy> {
    let relevant: Vec<Strategy> = extracted
        .iter()
        .filter(|s| {
            (s.problem_type == problem_type || s.problem_type == "unknown") && s.confidence >= 0.6
        })
        .cloned()
        .collect();
    if !relevant.is_empty() {
        return relevant;
    }
    let mut sorted = extracted.to_vec();
    sorted.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(2);
    sorted
}

async fn enhance(
    ctx: &Ctx,
    original: &AgentSolution,
    problem: &str,
    peers: &[Strategy],
    max_tokens: &Value,
) -> Option<AgentSolution> {
    let mut insights = String::new();
    for s in peers.iter().take(2) {
        insights += &format!("\nPeer Strategy from Agent {}:\n", s.agent_id);
        insights += &format!("- Approach: {}\n", s.approach_type);
        insights += &format!(
            "- Key Insights: {}\n",
            s.key_insights
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
        insights += &format!(
            "- Techniques: {}\n",
            s.techniques
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
        insights += &format!(
            "- Success Pattern: {}...\n",
            py::strings::prefix(&s.solution_pattern, 200)
        );
    }
    #[rustfmt::skip]
    let prompt = format!(
        concat!(
            "You are Agent {p0} collaborating with other mathematical agents.\n",
            "\n",
            "Original Problem: {p1}\n",
            "\n",
            "Your Current Solution:\n",
            "{p2}\n",
            "\n",
            "Peer Agent Strategy Insights:\n",
            "{p3}\n",
            "\n",
            "Task: Enhance your solution by incorporating the most valuable insights from your peers while maintaining your unique approach. Consider:\n",
            "\n",
            "1. Can any peer techniques strengthen your solution?\n",
            "2. Do peer insights reveal gaps in your reasoning?\n",
            "3. Can you combine approaches for a more robust solution?\n",
            "4. What verification steps from peers could improve confidence?\n",
            "\n",
            "Provide an enhanced solution that synthesizes the best ideas while ensuring mathematical rigor.\n",
            "\n",
            "Enhanced Solution:",
        ),
        p0 = original.agent_id,
        p1 = problem,
        p2 = original.solution,
        p3 = insights,
    );
    let response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [
                msg("system", "You are a collaborative mathematical agent learning from peer insights."),
                msg("user", &prompt)
            ],
            "max_tokens": max_tokens,
            "temperature": original.temperature * 0.9,
            "extra_body": {"reasoning": {"effort": "high"}}
        }))
        .await
        .ok()?;
    let text = py::strings::strip(&response.content_at(0).ok()??).to_string();
    let (reasoning, _) = reasoning_and_total(&response);
    Some(AgentSolution {
        agent_id: format!("enhanced_{}", original.agent_id),
        solution: text,
        confidence: py_min(original.confidence + 0.1, 1.0),
        reasoning_tokens: reasoning,
        is_verified: false,
        verification_score: 0.0,
        temperature: original.temperature,
        verification_results: Vec::new(),
    })
}

/// `share_strategies_across_agents`.
pub async fn share_strategies(
    ctx: &Ctx,
    ws: &Mutex<Workspace>,
    extracted: &[Strategy],
    executor: &Executor,
) -> Result<()> {
    let problem = ws.lock().unwrap().problem.clone();
    let problem_type = executor.run(classify_problem(ctx, &problem)).await;
    let effective = effective_strategies(&problem_type, extracted);
    let (solutions, max_tokens) = {
        let w = ws.lock().unwrap();
        (
            w.solutions.clone(),
            w.config.get_or("max_tokens", json!(30000)),
        )
    };
    let mut jobs = Vec::new();
    for s in solutions.iter().filter(|s| !s.agent_id.starts_with("agg_")) {
        let peers: Vec<Strategy> = effective
            .iter()
            .filter(|p| p.agent_id != s.agent_id)
            .cloned()
            .collect();
        if !peers.is_empty() {
            jobs.push((s.clone(), peers));
        }
    }
    let results = futures::future::join_all(
        jobs.iter()
            .map(|(s, peers)| executor.run(enhance(ctx, s, &problem, peers, &max_tokens))),
    )
    .await;
    let mut w = ws.lock().unwrap();
    for enhanced in results.into_iter().flatten() {
        w.add_solution(enhanced)?;
    }
    Ok(())
}
