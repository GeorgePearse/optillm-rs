//! Port of upstream `optillm/mcts.py` (Monte Carlo Tree Search over dialogue states).
//!
//! `random.choice` calls go through the request's CPython-compatible RNG so a
//! seeded run makes the same choices as upstream.

use anyhow::anyhow;
use serde_json::{json, Value};

use super::{min_tokens, py_max, py_min, ApproachResult, Ctx};
use crate::parse::msg;
use crate::py;

#[derive(Clone)]
struct DialogueState {
    system_prompt: String,
    conversation_history: Vec<Value>,
    current_query: String,
}

struct Node {
    state: DialogueState,
    parent: Option<usize>,
    children: Vec<usize>,
    visits: f64,
    value: f64,
}

struct Mcts<'a> {
    ctx: &'a Ctx,
    simulation_depth: i64,
    exploration_weight: f64,
    max_tokens: Value,
    nodes: Vec<Node>,
    completion_tokens: i64,
}

impl<'a> Mcts<'a> {
    fn select(&self, node: usize) -> usize {
        let n = &self.nodes[node];
        if n.children.is_empty() {
            return node;
        }
        let parent_visits = n.visits;
        let scores: Vec<f64> = n
            .children
            .iter()
            .map(|&c| {
                let c = &self.nodes[c];
                c.value / (c.visits + 1e-8)
                    + self.exploration_weight
                        * ((parent_visits + 1.0).ln() / (c.visits + 1e-8)).sqrt()
            })
            .collect();
        let best = py::strings::argmax_by(&scores, |s| *s).unwrap_or(0);
        n.children[best]
    }

    async fn expand(&mut self, node: usize) -> anyhow::Result<usize> {
        let state = self.nodes[node].state.clone();
        let actions = self.generate_actions(&state).await?;
        for action in &actions {
            let new_state = self.apply_action(&state, action).await?;
            self.nodes.push(Node {
                state: new_state,
                parent: Some(node),
                children: Vec::new(),
                visits: 0.0,
                value: 0.0,
            });
            let id = self.nodes.len() - 1;
            self.nodes[node].children.push(id);
        }
        let children = &self.nodes[node].children;
        let pick = self
            .ctx
            .rng()
            .choice_index(children.len())
            .ok_or_else(|| anyhow!("Cannot choose from an empty sequence"))?;
        Ok(children[pick])
    }

    async fn simulate(&mut self, node: usize) -> anyhow::Result<f64> {
        let mut state = self.nodes[node].state.clone();
        for _ in 0..self.simulation_depth.max(0) {
            if is_terminal(&state) {
                break;
            }
            let actions = self.generate_actions(&state).await?;
            let pick = self
                .ctx
                .rng()
                .choice_index(actions.len())
                .ok_or_else(|| anyhow!("Cannot choose from an empty sequence"))?;
            state = self.apply_action(&state, &actions[pick]).await?;
        }
        self.evaluate_state(&state).await
    }

    fn backpropagate(&mut self, mut node: Option<usize>, value: f64) {
        while let Some(n) = node {
            self.nodes[n].visits += 1.0;
            self.nodes[n].value += value;
            node = self.nodes[n].parent;
        }
    }

    async fn search(
        &mut self,
        initial_state: DialogueState,
        num_simulations: i64,
    ) -> anyhow::Result<DialogueState> {
        self.nodes.push(Node {
            state: initial_state,
            parent: None,
            children: Vec::new(),
            visits: 0.0,
            value: 0.0,
        });
        for _ in 0..num_simulations.max(0) {
            let mut node = self.select(0);
            if !is_terminal(&self.nodes[node].state) {
                node = self.expand(node).await?;
            }
            let value = self.simulate(node).await?;
            self.backpropagate(Some(node), value);
        }
        let children = &self.nodes[0].children;
        let visits: Vec<f64> = children.iter().map(|&c| self.nodes[c].visits).collect();
        let best = py::strings::argmax_by(&visits, |v| *v)
            .ok_or_else(|| anyhow!("max() iterable argument is empty"))?;
        Ok(self.nodes[children[best]].state.clone())
    }

    fn base_messages(state: &DialogueState) -> Vec<Value> {
        let mut messages = vec![msg("system", &state.system_prompt)];
        messages.extend(state.conversation_history.iter().cloned());
        messages
    }

    async fn generate_actions(&mut self, state: &DialogueState) -> anyhow::Result<Vec<String>> {
        let mut messages = Self::base_messages(state);
        messages.push(msg("user", &state.current_query));
        let response = self
            .ctx
            .create(json!({
                "model": self.ctx.model,
                "messages": messages,
                "max_tokens": self.max_tokens,
                "n": 3,
                "temperature": 1
            }))
            .await?;
        if response.choices().is_empty() {
            return Ok(Vec::new());
        }
        let completions: Vec<String> = (0..response.choices().len())
            .filter_map(|i| response.content(i))
            .map(|c| py::strings::strip(&c).to_string())
            .collect();
        self.completion_tokens += response.completion_tokens()?;
        Ok(completions)
    }

    async fn apply_action(
        &mut self,
        state: &DialogueState,
        action: &str,
    ) -> anyhow::Result<DialogueState> {
        let mut new_history = state.conversation_history.clone();
        new_history.push(msg("assistant", action));
        let mut messages = vec![msg("system", &state.system_prompt)];
        messages.extend(new_history.iter().cloned());
        messages.push(msg(
            "user",
            "Based on this conversation, what might the user ask or say next? Provide a likely user query.",
        ));
        let response = self
            .ctx
            .create(json!({
                "model": self.ctx.model,
                "messages": messages,
                "max_tokens": min_tokens(&self.max_tokens, 1024),
                "n": 1,
                "temperature": 1
            }))
            .await?;
        let next_query = if response.is_unusable() {
            "Please continue.".to_string()
        } else {
            response.content(0).unwrap_or_default()
        };
        self.completion_tokens += response.completion_tokens()?;
        Ok(DialogueState {
            system_prompt: state.system_prompt.clone(),
            conversation_history: new_history,
            current_query: next_query,
        })
    }

    async fn evaluate_state(&mut self, state: &DialogueState) -> anyhow::Result<f64> {
        let mut messages = Self::base_messages(state);
        messages.push(msg("user", "Evaluate the quality of this conversation on a scale from 0 to 1, where 0 is poor and 1 is excellent. Consider factors such as coherence, relevance, and engagement. Respond with only a number."));
        let response = self
            .ctx
            .create(json!({
                "model": self.ctx.model,
                "messages": messages,
                "max_tokens": 256,
                "n": 1,
                "temperature": 0.1
            }))
            .await?;
        self.completion_tokens += response.completion_tokens()?;
        if response.is_unusable() {
            return Ok(0.5);
        }
        let content = response.content(0).unwrap_or_default();
        Ok(
            match py::strings::parse_float(py::strings::strip(&content)) {
                Some(score) => py_max(0.0, py_min(score, 1.0)),
                None => 0.5,
            },
        )
    }
}

fn is_terminal(state: &DialogueState) -> bool {
    state.conversation_history.len() > 10 || state.current_query.to_lowercase().contains("goodbye")
}

fn config_f64(ctx: &Ctx, key: &str, default: f64) -> f64 {
    ctx.request_config
        .get(key)
        .and_then(|v| v.as_f64())
        .unwrap_or(default)
}

fn config_int(ctx: &Ctx, key: &str, default: i64) -> i64 {
    ctx.request_config
        .get(key)
        .and_then(|v| v.as_i64())
        .unwrap_or(default)
}

pub async fn chat_with_mcts(system_prompt: &str, initial_query: &str, ctx: &Ctx) -> ApproachResult {
    let num_simulations = config_int(ctx, "mcts_simulations", ctx.config.mcts_simulations);
    let exploration_weight = config_f64(ctx, "mcts_exploration", ctx.config.mcts_exploration);
    let simulation_depth = config_int(ctx, "mcts_depth", ctx.config.mcts_depth);
    let mut mcts = Mcts {
        ctx,
        simulation_depth,
        exploration_weight,
        max_tokens: ctx.max_tokens(4096),
        nodes: Vec::new(),
        completion_tokens: 0,
    };
    let initial_state = DialogueState {
        system_prompt: system_prompt.to_string(),
        conversation_history: Vec::new(),
        current_query: initial_query.to_string(),
    };
    let final_state = mcts.search(initial_state, num_simulations).await?;
    let response = final_state
        .conversation_history
        .last()
        .and_then(|m| m.get("content"))
        .map(py::json::to_str)
        .unwrap_or_default();
    Ok((Value::String(response), mcts.completion_tokens))
}
