//! Port of upstream `optillm/rstar.py` (rStar: MCTS with mutual consistency).
//!
//! Upstream's `solve()` runs `solve_async`, whose `num_rollouts` rollouts run
//! concurrently under `asyncio.gather` and touch the shared tree only between
//! awaits. The rollouts here are futures polled together by `join_all`, with
//! the tree behind a mutex that is never held across an await, so they
//! interleave at exactly the same points.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Value};

use super::{ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

const ACTIONS: [&str; 5] = ["A1", "A2", "A3", "A4", "A5"];

struct Node {
    state: String,
    parent: Option<usize>,
    children: Vec<usize>,
    visits: i64,
    value: f64,
}

struct RStar<'a> {
    ctx: &'a Ctx,
    max_depth: i64,
    num_rollouts: i64,
    c: f64,
    original_question: String,
    max_tokens: Value,
    tokens: AtomicI64,
    tree: Mutex<Vec<Node>>,
}

/// `RStar.extract_answer`.
pub fn extract_answer(final_state: &str) -> (String, f64) {
    let patterns = [
        r"The answer is (\d+)",
        r"The final answer is (\d+)",
        r"Therefore, the answer is (\d+)",
        r"So, the answer is (\d+)",
        r"Thus, the answer is (\d+)",
        r"In conclusion, the answer is (\d+)",
    ];
    for p in patterns {
        if let Some(c) = Regex::new(p).unwrap().captures(final_state) {
            return (c[1].to_string(), 1.0);
        }
    }
    let numbers: Vec<&str> = Regex::new(r"\d+")
        .unwrap()
        .find_iter(final_state)
        .map(|m| m.as_str())
        .collect();
    if let Some(last) = numbers.last() {
        return (last.to_string(), 0.5);
    }
    (String::new(), 0.0)
}

fn normalized_words(s: &str) -> std::collections::HashSet<String> {
    py::strings::split_whitespace(&s.to_lowercase().replace('.', "").replace(',', ""))
        .into_iter()
        .map(|w| w.to_string())
        .collect()
}

impl<'a> RStar<'a> {
    fn create_prompt(&self, state: &str, action: &str) -> String {
        let question = &self.original_question;
        #[rustfmt::skip]
        let prompt = match action {
            "A1" => format!(
                concat!(
                    "Given the current state: {p0}\n",
                    "Generate the next logical step in solving {p1}.\n",
                    "Your response should be a single, clear thought that moves towards the solution.\n",
                    "If you can determine the final answer at this step, state it clearly.",
                ),
                p0 = state, p1 = question,
            ),
            "A2" => format!(
                concat!(
                    "Given the current state: {p0}\n",
                    "Continue the reasoning process to solve {p1}.\n",
                    "Provide the remaining steps needed to reach the final answer.\n",
                    "Each step should be clear and directly related to solving the problem.",
                ),
                p0 = state, p1 = question,
            ),
            "A3" => format!(
                concat!(
                    "Given the current state: {p0}\n",
                    "Identify a key sub-question that needs to be answered to solve {p1}.\n",
                    "State this sub-question clearly, then provide its answer.\n",
                    "Explain how this sub-question and its answer contribute to solving the main problem.",
                ),
                p0 = state, p1 = question,
            ),
            "A4" => format!(
                concat!(
                    "Given the current state: {p0}\n",
                    "Re-examine the previous step in solving {p1} using Chain-of-Thought reasoning.\n",
                    "Break down your thinking process explicitly, showing each logical step.\n",
                    "If you reach a conclusion, state it clearly.",
                ),
                p0 = state, p1 = question,
            ),
            _ => format!(
                concat!(
                    "Given the current state: {p0}\n",
                    "Rephrase {p1} by clearly listing all relevant conditions and unknowns.\n",
                    "Ensure that your rephrasing captures all important details from the original question.\n",
                    "This rephrasing should help clarify the problem and guide the solution process.",
                ),
                p0 = state, p1 = question,
            ),
        };
        prompt + "\n\nIf you determine the final answer, explicitly state 'The final answer is [your numeric answer]' at the end of your response."
    }

    async fn generate_response(&self, prompt: &str) -> Result<String> {
        let response = self
            .ctx
            .create(json!({
                "model": self.ctx.model,
                "messages": [
                    msg("system", "You are a helpful assistant focused on solving mathematical problems. Stick to the given question and avoid introducing new scenarios."),
                    msg("user", prompt)
                ],
                "max_tokens": self.max_tokens,
                "temperature": 0.2
            }))
            .await?;
        self.tokens
            .fetch_add(response.completion_tokens()?, Ordering::SeqCst);
        let content = response
            .content_at(0)?
            .ok_or_else(|| anyhow!("'NoneType' object has no attribute 'strip'"))?;
        Ok(py::strings::strip(&content).to_string())
    }

    fn random_action(&self) -> &'static str {
        let i = self.ctx.rng().randbelow(ACTIONS.len());
        ACTIONS[i]
    }

    fn select_action(&self, node: usize) -> Result<usize> {
        let tree = self.tree.lock().unwrap();
        let n = &tree[node];
        if n.children.is_empty() {
            drop(tree);
            // random.choice(self.actions); the action is discarded by callers.
            self.random_action();
            return Ok(node);
        }
        let mut uct_values = Vec::new();
        for &c in &n.children {
            let child = &tree[c];
            let uct = if child.visits == 0 {
                f64::INFINITY
            } else {
                if n.visits <= 0 {
                    return Err(anyhow!("math domain error"));
                }
                child.value / child.visits as f64
                    + self.c * ((n.visits as f64).ln() / child.visits as f64).sqrt()
            };
            uct_values.push(uct);
        }
        let best = py::strings::argmax_by(&uct_values, |v| *v).unwrap_or(0);
        Ok(n.children[best])
    }

    async fn expand(&self, node: usize, action: &str) -> Result<usize> {
        let state = self.tree.lock().unwrap()[node].state.clone();
        let prompt = self.create_prompt(&state, action);
        let new_state = self.generate_response(&prompt).await?;
        let mut tree = self.tree.lock().unwrap();
        tree.push(Node {
            state: new_state,
            parent: Some(node),
            children: Vec::new(),
            visits: 0,
            value: 0.0,
        });
        let id = tree.len() - 1;
        tree[node].children.push(id);
        Ok(id)
    }

    fn evaluate(&self, node: usize) -> f64 {
        let state = self.tree.lock().unwrap()[node].state.clone();
        let (answer, confidence) = extract_answer(&state);
        if py::strings::parse_float(&answer).is_some() {
            confidence
        } else {
            0.0
        }
    }

    async fn simulate(&self, node: usize) -> Result<f64> {
        let mut current = node;
        let mut depth = 0;
        while depth < self.max_depth {
            let children = self.tree.lock().unwrap()[current].children.clone();
            if children.is_empty() {
                let action = self.random_action();
                current = self.expand(current, action).await?;
            } else {
                let i = self.ctx.rng().randbelow(children.len());
                current = children[i];
            }
            depth += 1;
        }
        Ok(self.evaluate(current))
    }

    fn backpropagate(&self, node: usize, value: f64) {
        let mut tree = self.tree.lock().unwrap();
        let mut cur = Some(node);
        while let Some(n) = cur {
            tree[n].visits += 1;
            tree[n].value += value;
            cur = tree[n].parent;
        }
    }

    async fn rollout(&self) -> Result<()> {
        let mut node = 0usize;
        while !self.tree.lock().unwrap()[node].children.is_empty() {
            node = self.select_action(node)?;
        }
        let action = self.random_action();
        if self.tree.lock().unwrap()[node].children.len() < ACTIONS.len() {
            node = self.expand(node, action).await?;
        }
        let value = self.simulate(node).await?;
        self.backpropagate(node, value);
        Ok(())
    }

    fn extract_trajectories(&self) -> Vec<Vec<usize>> {
        let tree = self.tree.lock().unwrap();
        let mut trajectories = Vec::new();
        let mut stack: Vec<(usize, Vec<usize>)> = vec![(0, Vec::new())];
        while let Some((node, path)) = stack.pop() {
            if tree[node].children.is_empty() {
                let mut t = path.clone();
                t.push(node);
                trajectories.push(t);
            } else {
                for &child in &tree[node].children {
                    let mut p = path.clone();
                    p.push(node);
                    stack.push((child, p));
                }
            }
        }
        trajectories
    }

    fn states(&self, nodes: &[usize]) -> Vec<String> {
        let tree = self.tree.lock().unwrap();
        nodes.iter().map(|&n| tree[n].state.clone()).collect()
    }

    async fn mutual_consistency(&self, trajectory: &[usize]) -> Result<bool> {
        let hi = trajectory.len() as i64 - 1;
        if hi < 1 {
            return Err(anyhow!("empty range in randrange(1, {})", hi + 1));
        }
        let split_index = self.ctx.rng().randint(1, hi) as usize;
        let partial = self.states(&trajectory[..split_index]).join(" ");
        let prompt = format!(
            "Given the partial reasoning:\n{}\nComplete the reasoning to solve the problem:",
            partial
        );
        let completion = self.generate_response(&prompt).await?;
        let remaining = self.states(&trajectory[split_index..]).join(" ");
        let a = normalized_words(&completion);
        let b = normalized_words(&remaining);
        let overlap = a.intersection(&b).count();
        let total = a.union(&b).count();
        if total == 0 {
            return Err(anyhow!("division by zero"));
        }
        Ok(overlap as f64 / total as f64 > 0.7)
    }

    fn trajectory_score(&self, trajectory: &[usize]) -> f64 {
        let tree = self.tree.lock().unwrap();
        match trajectory.last() {
            None => f64::NEG_INFINITY,
            Some(&last) => {
                let n = &tree[last];
                if n.visits == 0 {
                    n.value
                } else {
                    n.value / n.visits as f64
                }
            }
        }
    }
}

/// `RStar.select_best_answer`.
fn select_best_answer(answers: &[(String, f64)]) -> String {
    let mut counts: Vec<(String, (i64, f64))> = Vec::new();
    for (answer, conf) in answers.iter().filter(|(a, _)| !a.is_empty()) {
        if let Some(entry) = counts.iter_mut().find(|(a, _)| a == answer) {
            entry.1 = (entry.1 .0 + 1, super::py_max(entry.1 .1, *conf));
        } else {
            counts.push((answer.clone(), (1, *conf)));
        }
    }
    if counts.is_empty() {
        return "Unable to determine a valid answer.".to_string();
    }
    // sorted(key=lambda x: (-conf, -count)) is stable.
    counts.sort_by(|a, b| {
        (-a.1 .1)
            .partial_cmp(&-b.1 .1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then((-a.1 .0).cmp(&-b.1 .0))
    });
    counts[0].0.clone()
}

pub async fn rstar(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let _ = system_prompt; // upstream stores it but never uses it
    let solver = RStar {
        ctx,
        max_depth: ctx.config.rstar_max_depth,
        num_rollouts: ctx.config.rstar_num_rollouts,
        c: ctx.config.rstar_c,
        original_question: initial_query.to_string(),
        max_tokens: ctx.max_tokens(4096),
        tokens: AtomicI64::new(0),
        tree: Mutex::new(vec![Node {
            state: initial_query.to_string(),
            parent: None,
            children: Vec::new(),
            visits: 0,
            value: 0.0,
        }]),
    };
    let rollouts = (0..solver.num_rollouts.max(0)).map(|_| solver.rollout());
    for r in futures::future::join_all(rollouts).await {
        r?;
    }
    let trajectories = solver.extract_trajectories();

    let mut valid = Vec::new();
    for t in &trajectories {
        if solver.mutual_consistency(t).await? {
            valid.push(t.clone());
        }
    }
    let pool = if valid.is_empty() {
        &trajectories
    } else {
        &valid
    };
    let scores: Vec<f64> = pool.iter().map(|t| solver.trajectory_score(t)).collect();
    let best = py::strings::argmax_by(&scores, |s| *s)
        .ok_or_else(|| anyhow!("max() iterable argument is empty"))?;
    let final_trajectory = pool[best].clone();

    let answers: Vec<(String, f64)> = solver
        .states(&final_trajectory)
        .iter()
        .map(|s| extract_answer(s))
        .collect();
    let final_answer = select_best_answer(&answers);
    Ok((
        Value::String(final_answer),
        solver.tokens.load(Ordering::SeqCst),
    ))
}
