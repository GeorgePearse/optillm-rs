//! Port of upstream `optillm/pvg.py` (Prover-Verifier Game).
//!
//! Upstream keeps the completion-token counter in a module-level global that
//! is never reset, so the `completion_tokens` it reports grow across requests
//! for the lifetime of the server. That is replicated here (a process-wide
//! counter) so usage accounting matches.

use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::anyhow;
use regex::Regex;
use serde_json::{json, Value};

use super::{min_tokens, ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

static PVG_COMPLETION_TOKENS: AtomicI64 = AtomicI64::new(0);

fn add_tokens(n: i64) {
    PVG_COMPLETION_TOKENS.fetch_add(n, Ordering::SeqCst);
}

#[rustfmt::skip]
const ROLE_INSTRUCTION: &str = concat!(
    "\n",
    "    You are a problem solver tasked with generating solutions to the given problem. \n",
    "    If you are in 'helpful' mode:\n",
    "    - Provide a correct, detailed, and step-by-step solution to the problem.\n",
    "    - Explain your reasoning clearly and logically.\n",
    "    - Ensure your final answer or conclusion is accurate and well-supported.\n",
    "    - Use domain-specific terminology or notation if appropriate.\n",
    "\n",
    "    If you are in 'sneaky' mode:\n",
    "    - Provide a solution that appears correct but contains subtle errors or flaws.\n",
    "    - Make your explanation seem logical and convincing.\n",
    "    - Ensure your final answer or conclusion is incorrect, but in a way that's not immediately obvious.\n",
    "    - Maintain the appearance of expertise in the relevant domain.\n",
    "    ",
);

/// Message content for a Python value that may be `None`.
fn content_value(v: &Value) -> Value {
    v.clone()
}

async fn generate_solutions(
    ctx: &Ctx,
    system_prompt: &str,
    query: &Value,
    num_solutions: i64,
    is_sneaky: bool,
    temperature: f64,
    max_tokens: &Value,
) -> anyhow::Result<Vec<Value>> {
    let role = if is_sneaky { "sneaky" } else { "helpful" };
    let system = format!(
        "{}\n{}\nYou are in {} mode.",
        system_prompt, ROLE_INSTRUCTION, role
    );
    let response = ctx
        .create(json!({
            "model": ctx.model,
            "messages": [msg("system", &system), {"role": "user", "content": content_value(query)}],
            "n": num_solutions,
            "max_tokens": max_tokens,
            "temperature": temperature,
        }))
        .await?;
    add_tokens(response.completion_tokens()?);
    Ok((0..response.choices().len())
        .map(|i| {
            response
                .message(i)
                .and_then(|m| m.get("content"))
                .cloned()
                .unwrap_or(Value::Null)
        })
        .collect())
}

async fn verify_solutions(
    ctx: &Ctx,
    system_prompt: &str,
    initial_query: &Value,
    solutions: &[Value],
) -> anyhow::Result<Vec<f64>> {
    #[rustfmt::skip]
    let verify_prompt = format!(
        concat!(
            "{p0}\n",
            "You are a verifier tasked with evaluating the correctness and clarity of solutions to the given problem.\n",
            "Rate the following solution on a scale from 0 to 10, where:\n",
            "- 0 is completely incorrect or incomprehensible\n",
            "- 5 is partially correct or lacks clarity\n",
            "- 10 is perfectly correct, clear, and well-explained\n",
            "\n",
            "Consider the following criteria:\n",
            "1. Accuracy of the final answer or conclusion\n",
            "2. Correctness of each step or argument in the solution\n",
            "3. Clarity and logical flow of the explanation\n",
            "4. Appropriate use of domain-specific concepts or terminology\n",
            "5. Completeness of the solution\n",
            "\n",
            "Be especially vigilant for subtle errors or flaws that might not be immediately obvious.\n",
            "\n",
            "Provide your response in the following format:\n",
            "\n",
            "Score: [Your numerical score from 0 to 10]\n",
            "Explanation: [Your detailed explanation for the score, highlighting specific strengths or weaknesses]\n",
            "\n",
            "Ensure that the Score is a single number between 0 and 10, and the Explanation is on a new line.",
        ),
        p0 = system_prompt,
    );
    let score_re = Regex::new(r"Score:\s*(\d+(\.\d+)?)").unwrap();
    let mut scores = Vec::new();
    for solution in solutions {
        let user = format!(
            "Problem: {}\n\nSolution: {}",
            py::json::to_str(initial_query),
            py::json::to_str(solution)
        );
        let response = ctx
            .create(json!({
                "model": ctx.model,
                "messages": [msg("system", &verify_prompt), msg("user", &user)],
                "max_tokens": 1024,
                "temperature": 0.2,
            }))
            .await?;
        add_tokens(response.completion_tokens()?);
        let rating = response
            .content_at(0)?
            .ok_or_else(|| anyhow!("expected string or bytes-like object, got 'NoneType'"))?;
        let score = score_re
            .captures(&rating)
            .and_then(|c| py::strings::parse_float(c.get(1).unwrap().as_str()))
            .unwrap_or(0.0);
        scores.push(score);
    }
    Ok(scores)
}

pub async fn inference_time_pv_game(
    system_prompt: &str,
    initial_query: &str,
    ctx: &Ctx,
) -> ApproachResult {
    let num_rounds = 2;
    let num_solutions = 3;
    let max_tokens = ctx.max_tokens(4096);
    let mut query = Value::String(initial_query.to_string());
    let mut best_solution = Value::String(String::new());
    let mut best_score = -1.0f64;

    for round in 0..num_rounds {
        let temperature = f64::max(0.2, 0.7 - (round as f64 * 0.1));
        let helpful = generate_solutions(
            ctx,
            system_prompt,
            &query,
            num_solutions,
            false,
            temperature,
            &max_tokens,
        )
        .await?;
        let sneaky = generate_solutions(
            ctx,
            system_prompt,
            &query,
            num_solutions,
            true,
            temperature,
            &max_tokens,
        )
        .await?;
        let mut all_solutions = helpful;
        all_solutions.extend(sneaky);

        let scores = verify_solutions(ctx, system_prompt, &query, &all_solutions).await?;
        // max(zip(all_solutions, scores), key=lambda x: x[1]) on an empty zip raises.
        let idx = py::strings::argmax_by(&scores[..scores.len().min(all_solutions.len())], |s| *s)
            .ok_or_else(|| anyhow!("max() iterable argument is empty"))?;
        if scores[idx] > best_score {
            best_solution = all_solutions[idx].clone();
            best_score = scores[idx];
        }

        if round < num_rounds - 1 {
            #[rustfmt::skip]
            let refine_prompt = format!(
                concat!(
                    "\n",
                    "            Based on the original query and the best solution so far, suggest a refined query that might lead to an even better solution.\n",
                    "            Focus on aspects of the problem that were challenging or not fully addressed in the best solution.\n",
                    "            Maintain the core intent of the original query while adding specificity or context that could improve the solution.\n",
                    "            \n",
                    "            Original query: {p0}\n",
                    "            \n",
                    "            Best solution so far: {p1}\n",
                    "            \n",
                    "            Refined query:\n",
                    "            ",
                ),
                p0 = py::json::to_str(&query),
                p1 = py::json::to_str(&best_solution),
            );
            let response = ctx
                .create(json!({
                    "model": ctx.model,
                    "messages": [msg("system", system_prompt), msg("user", &refine_prompt)],
                    "max_tokens": min_tokens(&max_tokens, 1024),
                    "temperature": 0.5,
                }))
                .await?;
            add_tokens(response.completion_tokens()?);
            if response.choices().is_empty() {
                return Err(anyhow!("list index out of range"));
            }
            query = response
                .message(0)
                .and_then(|m| m.get("content"))
                .cloned()
                .unwrap_or(Value::Null);
        }
    }

    Ok((best_solution, PVG_COMPLETION_TOKENS.load(Ordering::SeqCst)))
}
