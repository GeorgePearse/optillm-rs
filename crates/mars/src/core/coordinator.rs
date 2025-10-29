/// Main coordinator that orchestrates the complete MARS execution.
///
/// Implements all 5 phases:
/// 1. Multi-Agent Exploration
/// 2a. RSA Aggregation (optional)
/// 2b. Strategy Network (optional)
/// 3. Verification System
/// 4. Iterative Improvement
/// 5. Final Synthesis

use crate::core::aggregator::Aggregator;
use crate::core::agent::Agent;
use crate::config::MarsConfig;
use crate::core::strategy::StrategyNetwork;
use crate::types::{MarsEvent, MarsOutput, SelectionMethod};
use crate::core::verifier::Verifier;
use crate::core::workspace::Workspace;
use crate::Result;
use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Coordinator for MARS execution
pub struct MarsCoordinator {
    config: MarsConfig,
    workspace: Workspace,
    strategy_network: StrategyNetwork,
}

impl MarsCoordinator {
    /// Create a new coordinator with default configuration
    pub fn new(config: MarsConfig) -> Self {
        Self {
            config,
            workspace: Workspace::new(),
            strategy_network: StrategyNetwork::new(),
        }
    }

    /// Run the complete MARS process for a given query
    ///
    /// Returns a stream of events and the final output
    pub async fn run(&mut self, query: &str) -> Result<MarsOutput> {
        let (tx, _rx) = mpsc::channel::<MarsEvent>(100);

        // Phase 1: Multi-Agent Exploration
        self.phase_exploration(query, &tx).await?;

        // Phase 2: Aggregation and Strategy Network (optional)
        if self.config.enable_aggregation {
            self.phase_aggregation(&tx).await?;
        }

        if self.config.enable_strategy_network {
            self.phase_strategy_network(&tx).await?;
        }

        // Phase 3: Verification
        self.phase_verification(&tx).await?;

        // Phase 4: Iterative Improvement
        for iteration in 0..self.config.max_iterations {
            let any_improved = self.phase_improvement(iteration, &tx).await?;
            if !any_improved {
                break; // No improvements made, early exit
            }
        }

        // Phase 5: Final Synthesis
        let output = self.phase_synthesis(&tx).await?;

        Ok(output)
    }

    /// Phase 1: Multi-Agent Exploration
    ///
    /// Spawn N agents with diverse temperatures to explore different solution paths
    async fn phase_exploration(&mut self, query: &str, tx: &mpsc::Sender<MarsEvent>) -> Result<()> {
        let _result = tx.send(MarsEvent::ExplorationStarted {
            num_agents: self.config.num_agents,
        }).await;

        // Create agents with diverse temperatures
        let mut agents = Vec::new();
        for temp in &self.config.temperatures[..self.config.num_agents] {
            agents.push(Agent::new(*temp));
        }

        // Generate placeholder solutions for now
        // TODO: Integrate with ModelClient for actual LLM calls
        for (idx, agent) in agents.iter().enumerate() {
            let solution = crate::types::Solution::new(
                agent.id.clone(),
                format!("Agent {} analyzed the query: {}", idx + 1, query),
                format!("Placeholder answer from agent {}", idx + 1),
                agent.temperature,
                0,
            );

            let _result = tx.send(MarsEvent::SolutionGenerated {
                solution_id: solution.id.clone(),
                agent_id: solution.agent_id.clone(),
            }).await;

            self.workspace.add_solution(solution).await;
        }

        Ok(())
    }

    /// Phase 2a: RSA-Inspired Aggregation (optional)
    async fn phase_aggregation(&mut self, tx: &mpsc::Sender<MarsEvent>) -> Result<()> {
        let _result = tx.send(MarsEvent::AggregationStarted).await;

        let solutions = self.workspace.get_all_solutions().await;

        let rsa_config = crate::RSAConfig::new(
            self.config.aggregation_population_size,
            self.config.aggregation_selection_size,
            self.config.aggregation_loops,
        );

        let aggregated = Aggregator::aggregate_rsa(&solutions, rsa_config)?;

        for solution in aggregated {
            let _result = tx.send(MarsEvent::SolutionsAggregated {
                result_solution_id: solution.id.clone(),
            }).await;

            self.workspace.add_solution(solution).await;
        }

        Ok(())
    }

    /// Phase 2b: Strategy Network (optional)
    async fn phase_strategy_network(&mut self, tx: &mpsc::Sender<MarsEvent>) -> Result<()> {
        let _result = tx.send(MarsEvent::StrategyNetworkStarted).await;

        let solutions = self.workspace.get_all_solutions().await;

        // Placeholder strategies for now
        // TODO: Integrate with ModelClient for actual strategy extraction
        for solution in solutions {
            let strategies = vec![
                "Use systematic reasoning".to_string(),
                "Break problem into components".to_string(),
            ];

            for strategy_desc in strategies {
                let strategy_id = self.strategy_network.register_strategy(
                    solution.agent_id.clone(),
                    strategy_desc.clone(),
                    format!("Strategy from solution {}", solution.id),
                );

                let _result = tx.send(MarsEvent::StrategyExtracted {
                    strategy_id,
                }).await;
            }
        }

        Ok(())
    }

    /// Phase 3: Verification System
    ///
    /// Cross-agent verification of all solutions
    async fn phase_verification(&mut self, tx: &mpsc::Sender<MarsEvent>) -> Result<()> {
        let _result = tx.send(MarsEvent::VerificationStarted).await;

        let solutions = self.workspace.get_all_solutions().await;

        for solution in solutions {
            // Create agents for verification (can be different from solution agents)
            let verifier_agents: Vec<_> = (0..2)
                .map(|_| Agent::new(0.3)) // Use low temperature for verification
                .collect();

            for (_pass_count, verifier) in verifier_agents.iter().enumerate() {
                match Verifier::verify_solution(&solution, &verifier.id).await {
                    Ok(verification_result) => {
                        let mut updated_solution = solution.clone();

                        if verification_result.is_correct {
                            updated_solution.add_verification_pass(verification_result.score);
                        } else {
                            updated_solution.add_verification_failure();
                        }

                        let _result = tx.send(MarsEvent::SolutionVerified {
                            solution_id: solution.id.clone(),
                            is_correct: verification_result.is_correct,
                            score: verification_result.score,
                        }).await;

                        let _ = self.workspace.update_solution(updated_solution).await;
                    }
                    Err(e) => {
                        let _result = tx.send(MarsEvent::Error {
                            message: format!("Verification failed: {}", e),
                        }).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Phase 4: Iterative Improvement
    ///
    /// Improve unverified solutions based on feedback
    async fn phase_improvement(
        &mut self,
        iteration: usize,
        tx: &mpsc::Sender<MarsEvent>,
    ) -> Result<bool> {
        let _result = tx.send(MarsEvent::ImprovementStarted {
            iteration,
        }).await;

        let solutions = self.workspace.get_all_solutions().await;
        let unverified: Vec<_> = solutions
            .iter()
            .filter(|s| !s.is_verified && s.verification_failures < 2)
            .collect();

        if unverified.is_empty() {
            return Ok(false); // No improvements possible
        }

        let mut improvements_made = false;

        for solution in unverified {
            // Placeholder improvement for now
            // TODO: Integrate with ModelClient for actual improvement
            let mut improved = solution.clone();
            improved.id = Uuid::new_v4().to_string();
            improved.phase = crate::types::GenerationPhase::Improved;
            improved.answer = format!("Improved: {}", improved.answer);

            let _result = tx.send(MarsEvent::SolutionImproved {
                solution_id: improved.id.clone(),
            }).await;

            self.workspace.add_solution(improved).await;
            improvements_made = true;
        }

        Ok(improvements_made)
    }

    /// Phase 5: Final Synthesis
    ///
    /// Select the best answer using consensus voting, verification score, or synthesis
    async fn phase_synthesis(&self, tx: &mpsc::Sender<MarsEvent>) -> Result<MarsOutput> {
        let _result = tx.send(MarsEvent::SynthesisStarted).await;

        let all_solutions = self.workspace.get_all_solutions().await;

        // Try consensus voting
        if let Some(final_solution) =
            self.select_by_majority_voting(&all_solutions) {
            let _result = tx.send(MarsEvent::AnswerSynthesized {
                answer: final_solution.answer.clone(),
            }).await;

            return Ok(self.create_output(
                all_solutions,
                final_solution,
                SelectionMethod::MajorityVoting,
            ));
        }

        // Try best verified solution
        if let Some(final_solution) = self.select_best_verified(&all_solutions) {
            let _result = tx.send(MarsEvent::AnswerSynthesized {
                answer: final_solution.answer.clone(),
            }).await;

            return Ok(self.create_output(
                all_solutions,
                final_solution,
                SelectionMethod::BestVerified,
            ));
        }

        // Fallback: use synthesized answer from top solutions
        let final_solution = self.synthesize_final_answer(&all_solutions)?;
        let _result = tx.send(MarsEvent::AnswerSynthesized {
            answer: final_solution.answer.clone(),
        }).await;

        Ok(self.create_output(
            all_solutions,
            final_solution,
            SelectionMethod::Synthesized,
        ))
    }

    /// Select answer by majority voting
    fn select_by_majority_voting(&self, solutions: &[crate::types::Solution]) -> Option<crate::types::Solution> {
        if solutions.len() < 2 {
            return solutions.first().cloned();
        }

        let mut answer_counts: std::collections::HashMap<String, usize> = Default::default();
        for sol in solutions {
            *answer_counts.entry(sol.answer.clone()).or_insert(0) += 1;
        }

        // Return answer that appears 2+ times
        for (answer, count) in answer_counts {
            if count >= 2 {
                return solutions.iter().find(|s| s.answer == answer).cloned();
            }
        }

        None
    }

    /// Select best verified solution
    fn select_best_verified(&self, solutions: &[crate::types::Solution]) -> Option<crate::types::Solution> {
        Verifier::find_best_verified(solutions)
    }

    /// Synthesize final answer from top solutions
    fn synthesize_final_answer(&self, solutions: &[crate::types::Solution]) -> Result<crate::types::Solution> {
        if solutions.is_empty() {
            return Err(crate::MarsError::NoSolutions);
        }

        let mut sorted = solutions.to_vec();
        sorted.sort_by(|a, b| {
            b.verification_score
                .partial_cmp(&a.verification_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let top_3: Vec<_> = sorted.iter().take(3).collect();

        let combined_reasoning = top_3
            .iter()
            .enumerate()
            .map(|(i, s)| format!("Approach {}:\n{}", i + 1, s.reasoning))
            .collect::<Vec<_>>()
            .join("\n\n");

        let final_answer = top_3
            .first()
            .map(|s| s.answer.clone())
            .unwrap_or_default();

        Ok(crate::types::Solution::new(
            "synthesizer".to_string(),
            combined_reasoning,
            final_answer,
            0.5,
            solutions.iter().map(|s| s.token_count).sum(),
        ))
    }

    /// Create the final output
    fn create_output(
        &self,
        all_solutions: Vec<crate::types::Solution>,
        final_solution: crate::types::Solution,
        selection_method: SelectionMethod,
    ) -> MarsOutput {
        let final_solution_id = final_solution.id.clone();
        let answer = final_solution.answer.clone();
        let reasoning = final_solution.reasoning.clone();

        MarsOutput {
            answer,
            reasoning,
            all_solutions,
            verifications: Vec::new(),
            final_solution_id,
            selection_method,
            iterations: 0,
            total_tokens: 0,
            completed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let config = MarsConfig::default();
        let coordinator = MarsCoordinator::new(config);
        assert_eq!(coordinator.config.num_agents, 3);
    }

    #[tokio::test]
    async fn test_majority_voting() {
        let config = MarsConfig::default();
        let coordinator = MarsCoordinator::new(config);

        let sol1 = crate::types::Solution::new(
            "agent1".to_string(),
            "r1".to_string(),
            "42".to_string(),
            0.3,
            100,
        );
        let sol2 = crate::types::Solution::new(
            "agent2".to_string(),
            "r2".to_string(),
            "42".to_string(),
            0.6,
            100,
        );
        let sol3 = crate::types::Solution::new(
            "agent3".to_string(),
            "r3".to_string(),
            "43".to_string(),
            1.0,
            100,
        );

        let solutions = vec![sol1, sol2, sol3];
        let selected = coordinator.select_by_majority_voting(&solutions);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().answer, "42");
    }
}
