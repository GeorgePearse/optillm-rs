/// Shared workspace for storing and managing solutions across agents.

use crate::types::Solution;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared workspace for all agents to store and access solutions
#[derive(Clone)]
pub struct Workspace {
    solutions: Arc<RwLock<Vec<Solution>>>,
}

impl Workspace {
    /// Create a new workspace
    pub fn new() -> Self {
        Self {
            solutions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a solution to the workspace
    pub async fn add_solution(&self, solution: Solution) {
        let mut solutions = self.solutions.write().await;
        solutions.push(solution);
    }

    /// Get all solutions in the workspace
    pub async fn get_all_solutions(&self) -> Vec<Solution> {
        let solutions = self.solutions.read().await;
        solutions.clone()
    }

    /// Get a specific solution by ID
    pub async fn get_solution(&self, id: &str) -> Option<Solution> {
        let solutions = self.solutions.read().await;
        solutions.iter().find(|s| s.id == id).cloned()
    }

    /// Update a solution in the workspace
    pub async fn update_solution(&self, updated_solution: Solution) -> crate::Result<()> {
        let mut solutions = self.solutions.write().await;
        if let Some(pos) = solutions.iter().position(|s| s.id == updated_solution.id) {
            solutions[pos] = updated_solution;
            Ok(())
        } else {
            Err(crate::MarsError::CoordinatorError(
                format!("Solution {} not found", updated_solution.id),
            ))
        }
    }

    /// Get all verified solutions
    pub async fn get_verified_solutions(&self) -> Vec<Solution> {
        let solutions = self.solutions.read().await;
        solutions
            .iter()
            .filter(|s| s.is_verified)
            .cloned()
            .collect()
    }

    /// Get solutions sorted by verification score (descending)
    pub async fn get_solutions_by_score(&self) -> Vec<Solution> {
        let solutions = self.solutions.read().await;
        let mut sorted = solutions.clone();
        sorted.sort_by(|a, b| {
            b.verification_score
                .partial_cmp(&a.verification_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    /// Get solutions from a specific agent
    pub async fn get_solutions_by_agent(&self, agent_id: &str) -> Vec<Solution> {
        let solutions = self.solutions.read().await;
        solutions
            .iter()
            .filter(|s| s.agent_id == agent_id)
            .cloned()
            .collect()
    }

    /// Count total solutions
    pub async fn count_solutions(&self) -> usize {
        let solutions = self.solutions.read().await;
        solutions.len()
    }

    /// Clear all solutions (useful for testing)
    pub async fn clear(&self) {
        let mut solutions = self.solutions.write().await;
        solutions.clear();
    }

    /// Get the best unverified solution by answer length (simpler answers are often better)
    pub async fn get_best_unverified(&self) -> Option<Solution> {
        let solutions = self.solutions.read().await;
        solutions
            .iter()
            .filter(|s| !s.is_verified && s.verification_failures == 0)
            .min_by_key(|s| s.answer.len())
            .cloned()
    }

    /// Get top N solutions by verification score
    pub async fn get_top_n_verified(&self, n: usize) -> Vec<Solution> {
        let mut solutions = self.get_solutions_by_score().await;
        solutions.truncate(n);
        solutions
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_add_and_get_solution() {
        let workspace = Workspace::new();
        let solution = Solution::new(
            "agent1".to_string(),
            "reasoning".to_string(),
            "answer".to_string(),
            0.5,
            100,
        );

        workspace.add_solution(solution.clone()).await;
        assert_eq!(workspace.count_solutions().await, 1);

        let retrieved = workspace.get_solution(&solution.id).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_get_verified_solutions() {
        let workspace = Workspace::new();

        let mut sol1 = Solution::new(
            "agent1".to_string(),
            "reasoning".to_string(),
            "answer1".to_string(),
            0.5,
            100,
        );
        sol1.add_verification_pass(0.9);
        sol1.add_verification_pass(0.9);

        let sol2 = Solution::new(
            "agent2".to_string(),
            "reasoning".to_string(),
            "answer2".to_string(),
            0.5,
            100,
        );

        workspace.add_solution(sol1).await;
        workspace.add_solution(sol2).await;

        let verified = workspace.get_verified_solutions().await;
        assert_eq!(verified.len(), 1);
    }

    #[tokio::test]
    async fn test_solutions_by_agent() {
        let workspace = Workspace::new();

        let sol1 =
            Solution::new("agent1".to_string(), "r1".to_string(), "a1".to_string(), 0.5, 100);
        let sol2 =
            Solution::new("agent1".to_string(), "r2".to_string(), "a2".to_string(), 0.5, 100);
        let sol3 =
            Solution::new("agent2".to_string(), "r3".to_string(), "a3".to_string(), 0.5, 100);

        workspace.add_solution(sol1).await;
        workspace.add_solution(sol2).await;
        workspace.add_solution(sol3).await;

        let agent1_sols = workspace.get_solutions_by_agent("agent1").await;
        assert_eq!(agent1_sols.len(), 2);
    }
}
