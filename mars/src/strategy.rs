/// Strategy network for sharing successful reasoning approaches across agents.

use crate::types::Strategy;
use crate::Result;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Network of strategies shared across agents
pub struct StrategyNetwork {
    strategies: HashMap<String, Strategy>,
}

impl StrategyNetwork {
    /// Create a new strategy network
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
        }
    }

    /// Register a new strategy discovered by an agent
    pub fn register_strategy(
        &mut self,
        agent_id: String,
        description: String,
        technique: String,
    ) -> String {
        let strategy = Strategy {
            id: Uuid::new_v4().to_string(),
            description,
            technique,
            discovered_by: agent_id,
            success_rate: 0.5, // Start with neutral success rate
            discovered_at: Utc::now(),
        };

        let id = strategy.id.clone();
        self.strategies.insert(id.clone(), strategy);
        id
    }

    /// Get all registered strategies
    pub fn get_all_strategies(&self) -> Vec<Strategy> {
        self.strategies.values().cloned().collect()
    }

    /// Get strategies discovered by a specific agent
    pub fn get_strategies_by_agent(&self, agent_id: &str) -> Vec<Strategy> {
        self.strategies
            .values()
            .filter(|s| s.discovered_by == agent_id)
            .cloned()
            .collect()
    }

    /// Update strategy success rate
    pub fn update_success_rate(&mut self, strategy_id: &str, success: bool) -> Result<()> {
        if let Some(strategy) = self.strategies.get_mut(strategy_id) {
            // Exponential moving average with alpha=0.2
            let delta = if success { 1.0 } else { 0.0 };
            strategy.success_rate = strategy.success_rate * 0.8 + delta * 0.2;
            Ok(())
        } else {
            Err(crate::MarsError::StrategyExtractionError(
                format!("Strategy {} not found", strategy_id),
            ))
        }
    }

    /// Get top strategies by success rate
    pub fn get_top_strategies(&self, n: usize) -> Vec<Strategy> {
        let mut strategies = self.get_all_strategies();
        strategies.sort_by(|a, b| {
            b.success_rate
                .partial_cmp(&a.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        strategies.truncate(n);
        strategies
    }

    /// Format strategies for sharing with agents
    pub fn format_strategies_for_agents(&self) -> String {
        let top_strategies = self.get_top_strategies(5);

        if top_strategies.is_empty() {
            return "No strategies discovered yet.".to_string();
        }

        let mut formatted = String::from("Successful strategies discovered by other agents:\n\n");

        for (idx, strategy) in top_strategies.iter().enumerate() {
            formatted.push_str(&format!(
                "{}. {} (Success rate: {:.1}%)\n   Technique: {}\n\n",
                idx + 1,
                strategy.description,
                strategy.success_rate * 100.0,
                strategy.technique
            ));
        }

        formatted
    }

    /// Clear all strategies (useful for testing)
    pub fn clear(&mut self) {
        self.strategies.clear();
    }

    /// Get number of registered strategies
    pub fn count_strategies(&self) -> usize {
        self.strategies.len()
    }

    /// Analyze strategy diversity
    pub fn get_diversity_metrics(&self) -> StrategyDiversity {
        let strategies = self.get_all_strategies();
        let num_agents: std::collections::HashSet<_> =
            strategies.iter().map(|s| s.discovered_by.clone()).collect();

        let avg_success_rate = if !strategies.is_empty() {
            strategies.iter().map(|s| s.success_rate).sum::<f32>() / strategies.len() as f32
        } else {
            0.0
        };

        StrategyDiversity {
            total_strategies: strategies.len(),
            unique_agents: num_agents.len(),
            avg_success_rate,
        }
    }
}

impl Default for StrategyNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics about strategy diversity in the network
#[derive(Debug, Clone)]
pub struct StrategyDiversity {
    /// Total number of strategies registered
    pub total_strategies: usize,
    /// Number of agents that contributed strategies
    pub unique_agents: usize,
    /// Average success rate across all strategies
    pub avg_success_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_strategy() {
        let mut network = StrategyNetwork::new();
        let id = network.register_strategy(
            "agent1".to_string(),
            "Break problem into parts".to_string(),
            "Use systematic decomposition".to_string(),
        );

        assert!(!id.is_empty());
        assert_eq!(network.count_strategies(), 1);
    }

    #[test]
    fn test_get_strategies_by_agent() {
        let mut network = StrategyNetwork::new();
        network.register_strategy(
            "agent1".to_string(),
            "Strategy 1".to_string(),
            "Technique 1".to_string(),
        );
        network.register_strategy(
            "agent1".to_string(),
            "Strategy 2".to_string(),
            "Technique 2".to_string(),
        );
        network.register_strategy(
            "agent2".to_string(),
            "Strategy 3".to_string(),
            "Technique 3".to_string(),
        );

        let agent1_strategies = network.get_strategies_by_agent("agent1");
        assert_eq!(agent1_strategies.len(), 2);

        let agent2_strategies = network.get_strategies_by_agent("agent2");
        assert_eq!(agent2_strategies.len(), 1);
    }

    #[test]
    fn test_update_success_rate() {
        let mut network = StrategyNetwork::new();
        let id = network.register_strategy(
            "agent1".to_string(),
            "Strategy".to_string(),
            "Technique".to_string(),
        );

        let initial_rate = network.strategies[&id].success_rate;

        // Update with success
        network.update_success_rate(&id, true).unwrap();
        let updated_rate = network.strategies[&id].success_rate;
        assert!(updated_rate > initial_rate);

        // Update with failure
        network.update_success_rate(&id, false).unwrap();
        let failed_rate = network.strategies[&id].success_rate;
        assert!(failed_rate < updated_rate);
    }

    #[test]
    fn test_top_strategies() {
        let mut network = StrategyNetwork::new();
        let id1 = network.register_strategy(
            "agent1".to_string(),
            "Strategy 1".to_string(),
            "Technique".to_string(),
        );
        let id2 = network.register_strategy(
            "agent2".to_string(),
            "Strategy 2".to_string(),
            "Technique".to_string(),
        );

        // Boost second strategy
        for _ in 0..5 {
            network.update_success_rate(&id2, true).unwrap();
        }

        let top = network.get_top_strategies(1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].id, id2);
    }

    #[test]
    fn test_diversity_metrics() {
        let mut network = StrategyNetwork::new();
        network.register_strategy(
            "agent1".to_string(),
            "S1".to_string(),
            "T1".to_string(),
        );
        network.register_strategy(
            "agent2".to_string(),
            "S2".to_string(),
            "T2".to_string(),
        );

        let diversity = network.get_diversity_metrics();
        assert_eq!(diversity.total_strategies, 2);
        assert_eq!(diversity.unique_agents, 2);
    }

    #[test]
    fn test_format_strategies() {
        let mut network = StrategyNetwork::new();
        network.register_strategy(
            "agent1".to_string(),
            "Decomposition".to_string(),
            "Break into parts".to_string(),
        );

        let formatted = network.format_strategies_for_agents();
        assert!(formatted.contains("Decomposition"));
        assert!(formatted.contains("Break into parts"));
    }
}
