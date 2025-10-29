//! Monte Carlo Tree Search (MCTS) for reasoning exploration.
//!
//! This module implements MCTS adapted from optillm for exploring
//! dialogue/reasoning trees. It uses UCB-based selection to balance
//! exploration and exploitation of promising reasoning paths.
//!
//! # Algorithm
//!
//! 1. **Selection**: Traverse tree using UCB formula to select promising nodes
//! 2. **Expansion**: Generate N actions (LLM completions) and create child nodes
//! 3. **Simulation**: Rollout to depth D using random actions
//! 4. **Backpropagation**: Update visit counts and values up the tree
//!
//! # Example
//!
//! ```no_run
//! # use optillm_mars::mcts::*;
//! # use optillm_mars::Result;
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let config = MCTSConfig::default();
//! let mut mcts = MCTS::new(config);
//! let state = DialogueState::new(
//!     "You are helpful".to_string(),
//!     vec![],
//!     "What is 2+2?".to_string(),
//! );
//!
//! // let final_state = mcts.search(state, provider).await?;
//! # Ok(())
//! # }
//! ```

use crate::Result;
use rand::{seq::SliceRandom, Rng};

/// Configuration for MCTS algorithm
#[derive(Clone, Debug)]
pub struct MCTSConfig {
    /// How deep to simulate (default: 1)
    pub simulation_depth: usize,
    /// UCB exploration weight (default: 0.2)
    pub exploration_weight: f32,
    /// Number of MCTS iterations (default: 2)
    pub num_simulations: usize,
    /// Number of action completions to generate (default: 3)
    pub num_actions: usize,
    /// Temperature for action generation (default: 1.0)
    pub generation_temperature: f32,
    /// Temperature for evaluation (default: 0.1)
    pub evaluation_temperature: f32,
    /// Max conversation history length (default: 10)
    pub max_history_length: usize,
}

impl Default for MCTSConfig {
    fn default() -> Self {
        Self {
            simulation_depth: 1,
            exploration_weight: 0.2,
            num_simulations: 2,
            num_actions: 3,
            generation_temperature: 1.0,
            evaluation_temperature: 0.1,
            max_history_length: 10,
        }
    }
}

/// Represents a single message in dialogue history
#[derive(Clone, Debug)]
pub struct Message {
    /// Role: "user" or "assistant"
    pub role: String,
    /// Message content
    pub content: String,
}

impl Message {
    /// Create a new message
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

/// Represents a dialogue state in the MCTS tree
#[derive(Clone, Debug)]
pub struct DialogueState {
    /// System prompt for the conversation
    pub system_prompt: String,
    /// History of messages in the conversation
    pub conversation_history: Vec<Message>,
    /// Current user query
    pub current_query: String,
}

impl DialogueState {
    /// Create a new dialogue state
    pub fn new(system_prompt: String, history: Vec<Message>, query: String) -> Self {
        Self {
            system_prompt,
            conversation_history: history,
            current_query: query,
        }
    }
}

/// Node in the MCTS tree
#[derive(Clone)]
pub struct MCTSNode {
    /// The dialogue state at this node
    pub state: DialogueState,
    /// Parent node index (None for root)
    pub parent: Option<usize>,
    /// Child node indices
    pub children: Vec<usize>,
    /// Number of times this node was visited
    pub visits: usize,
    /// Cumulative value from simulations
    pub value: f32,
}

impl MCTSNode {
    /// Create a new MCTS node
    pub fn new(state: DialogueState, parent: Option<usize>) -> Self {
        Self {
            state,
            parent,
            children: Vec::new(),
            visits: 0,
            value: 0.0,
        }
    }
}

/// Monte Carlo Tree Search for reasoning exploration
pub struct MCTS {
    /// Configuration
    config: MCTSConfig,
    /// All nodes in the tree (root at index 0)
    nodes: Vec<MCTSNode>,
    /// Root node index
    root_idx: Option<usize>,
    /// Token usage tracking
    pub completion_tokens: usize,
}

impl MCTS {
    /// Create a new MCTS instance with given configuration
    pub fn new(config: MCTSConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            root_idx: None,
            completion_tokens: 0,
        }
    }

    /// Select most promising node using UCB formula
    fn select(&self, node_idx: usize) -> usize {
        let node = &self.nodes[node_idx];

        // If no children, return current node
        if node.children.is_empty() {
            return node_idx;
        }

        // UCB formula: value/visits + exploration * sqrt(ln(parent_visits)/visits)
        let parent_visits = node.visits as f32;
        let mut best_idx = node.children[0];
        let mut best_score = f32::NEG_INFINITY;

        for &child_idx in &node.children {
            let child = &self.nodes[child_idx];
            let epsilon = 1e-8;

            let exploitation = child.value / (child.visits as f32 + epsilon);
            let exploration = self.config.exploration_weight
                * ((parent_visits + 1.0).ln() / (child.visits as f32 + epsilon)).sqrt();

            let ucb_score = exploitation + exploration;

            if ucb_score > best_score {
                best_score = ucb_score;
                best_idx = child_idx;
            }
        }

        best_idx
    }

    /// Expand node by generating possible actions
    async fn expand(
        &mut self,
        node_idx: usize,
        _provider: &dyn crate::ModelClient,
    ) -> Result<usize> {
        let node = self.nodes[node_idx].clone();
        let actions = self.generate_actions(&node.state, _provider).await?;

        // Create child nodes for each action
        for action in actions {
            let new_state = self.apply_action(&node.state, &action, _provider).await?;
            let child = MCTSNode::new(new_state, Some(node_idx));
            let child_idx = self.nodes.len();
            self.nodes.push(child);
            self.nodes[node_idx].children.push(child_idx);
        }

        // Randomly select one child for simulation
        if !self.nodes[node_idx].children.is_empty() {
            let children = &self.nodes[node_idx].children;
            let mut rng = rand::thread_rng();
            let idx = *children.choose(&mut rng).unwrap();
            Ok(idx)
        } else {
            Ok(node_idx)
        }
    }

    /// Simulate from node to terminal state
    async fn simulate(
        &mut self,
        node_idx: usize,
        _provider: &dyn crate::ModelClient,
    ) -> Result<f32> {
        let mut state = self.nodes[node_idx].state.clone();

        for _ in 0..self.config.simulation_depth {
            if self.is_terminal(&state) {
                break;
            }

            let actions = self.generate_actions(&state, _provider).await?;
            if actions.is_empty() {
                break;
            }

            // Random action selection for simulation
            if !actions.is_empty() {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..actions.len());
                state = self.apply_action(&state, &actions[idx], _provider).await?;
            }
        }

        self.evaluate_state(&state, _provider).await
    }

    /// Backpropagate value up the tree
    fn backpropagate(&mut self, mut node_idx: usize, value: f32) {
        loop {
            let node = &mut self.nodes[node_idx];
            node.visits += 1;
            node.value += value;

            if let Some(parent_idx) = node.parent {
                node_idx = parent_idx;
            } else {
                break;
            }
        }
    }

    /// Generate possible actions using LLM
    async fn generate_actions(
        &mut self,
        state: &DialogueState,
        _provider: &dyn crate::ModelClient,
    ) -> Result<Vec<String>> {
        // Build prompt from state
        let mut prompt = String::new();

        for msg in &state.conversation_history {
            prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }

        prompt.push_str(&format!("user: {}", state.current_query));

        // Generate N completions at high temperature
        let mut actions = Vec::new();
        for _ in 0..self.config.num_actions {
            // Note: full implementation would use the provider to generate completions
            // For now, we stub this to compile
            actions.push(format!("action_{}", actions.len()));
        }

        Ok(actions)
    }

    /// Apply action to state and predict next query
    async fn apply_action(
        &mut self,
        state: &DialogueState,
        action: &str,
        _provider: &dyn crate::ModelClient,
    ) -> Result<DialogueState> {
        // Add assistant response to history
        let mut new_history = state.conversation_history.clone();
        new_history.push(Message::new("assistant", action));

        // Predict next user query
        let mut prompt = String::new();
        for msg in &new_history {
            prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }
        prompt.push_str("\nBased on this conversation, what might the user ask or say next? Provide a likely user query.");

        // Note: full implementation would use the provider
        let next_query = format!("next_query_based_on_{}", action);

        Ok(DialogueState::new(
            state.system_prompt.clone(),
            new_history,
            next_query,
        ))
    }

    /// Evaluate quality of dialogue state (0.0 to 1.0)
    async fn evaluate_state(
        &mut self,
        state: &DialogueState,
        _provider: &dyn crate::ModelClient,
    ) -> Result<f32> {
        let mut prompt = String::new();
        for msg in &state.conversation_history {
            prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }
        prompt.push_str(
            "\n\nEvaluate the quality of this conversation on a scale from 0 to 1, where 0 is poor and 1 is excellent. \
             Consider factors such as coherence, relevance, and engagement. Respond with only a number.",
        );

        // Note: full implementation would use the provider
        // For now return neutral score
        Ok(0.5)
    }

    /// Check if the current state is terminal (conversation should end)
    pub fn is_terminal(&self, state: &DialogueState) -> bool {
        state.conversation_history.len() > self.config.max_history_length
            || state.current_query.to_lowercase().contains("goodbye")
    }

    /// Run MCTS search and return best state
    pub async fn search(
        &mut self,
        initial_state: DialogueState,
        provider: &dyn crate::ModelClient,
    ) -> Result<DialogueState> {
        // Initialize root if needed
        if self.root_idx.is_none() {
            let root = MCTSNode::new(initial_state, None);
            self.nodes.push(root);
            self.root_idx = Some(0);
        }

        // Run simulations
        for _ in 0..self.config.num_simulations {
            // Selection: traverse to most promising node
            let mut node_idx = self.root_idx.unwrap();
            while !self.nodes[node_idx].children.is_empty() {
                node_idx = self.select(node_idx);
            }

            // Expansion: generate children if not terminal
            if !self.is_terminal(&self.nodes[node_idx].state.clone()) {
                node_idx = self.expand(node_idx, provider).await?;
            }

            // Simulation: rollout from selected node
            let value = self.simulate(node_idx, provider).await?;

            // Backpropagation: update values up the tree
            self.backpropagate(node_idx, value);
        }

        // Return best child (most visited)
        let root = &self.nodes[self.root_idx.unwrap()];
        if root.children.is_empty() {
            return Ok(root.state.clone());
        }

        let best_child_idx = root
            .children
            .iter()
            .max_by_key(|&&idx| self.nodes[idx].visits)
            .copied()
            .unwrap();

        Ok(self.nodes[best_child_idx].state.clone())
    }
}


#[cfg(test)]
mod tests;
