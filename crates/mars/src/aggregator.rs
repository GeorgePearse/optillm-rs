/// RSA-inspired aggregation for refining solutions.

use crate::types::{GenerationPhase, Solution};
use crate::Result;
use rand::seq::SliceRandom;
use std::collections::HashSet;

/// Aggregator that combines multiple solutions to produce refined ones
pub struct Aggregator;

impl Aggregator {
    /// Run RSA-inspired aggregation on solutions
    ///
    /// This process:
    /// 1. Maintains a population of N solutions
    /// 2. Selects K solutions for refinement
    /// 3. Repeats T times to iteratively improve
    pub async fn aggregate_rsa(
        solutions: &[Solution],
        population_size: usize,
        selection_size: usize,
        num_loops: usize,
    ) -> Result<Vec<Solution>> {
        let mut aggregated = Vec::new();

        // Ensure we have solutions to work with
        if solutions.is_empty() {
            return Ok(aggregated);
        }

        let mut population = solutions.to_vec();

        // Limit population to requested size
        if population.len() > population_size {
            population.truncate(population_size);
        }

        // Perform aggregation loops
        for loop_idx in 0..num_loops {
            let selected = Self::select_diverse_solutions(&population, selection_size)?;

            // Create aggregated solution from selected ones
            if !selected.is_empty() {
                let aggregated_solution = Self::synthesize_solution(&selected, loop_idx)?;
                aggregated.push(aggregated_solution);

                // Add back to population for next iteration
                population.push(aggregated[aggregated.len() - 1].clone());
            }
        }

        Ok(aggregated)
    }

    /// Select diverse solutions from the population
    ///
    /// This promotes diversity to explore different reasoning paths
    fn select_diverse_solutions(
        solutions: &[Solution],
        num_to_select: usize,
    ) -> Result<Vec<Solution>> {
        if solutions.is_empty() {
            return Ok(Vec::new());
        }

        let num_to_select = num_to_select.min(solutions.len());
        let mut rng = rand::thread_rng();
        let selected: Vec<Solution> = solutions
            .choose_multiple(&mut rng, num_to_select)
            .cloned()
            .collect();

        Ok(selected)
    }

    /// Synthesize a new solution from multiple selected solutions
    fn synthesize_solution(solutions: &[Solution], iteration: usize) -> Result<Solution> {
        if solutions.is_empty() {
            return Err(crate::MarsError::AggregationError(
                "No solutions to synthesize".to_string(),
            ));
        }

        // Use the first solution as base for aggregation
        let base = &solutions[0];

        // Combine reasoning from all solutions
        let combined_reasoning = Self::combine_reasoning(solutions);

        // Determine best answer (could be most common or from best solution)
        let answer = Self::select_best_answer(solutions);

        let mut aggregated = Solution::new(
            format!("aggregator-iteration-{}", iteration),
            combined_reasoning,
            answer,
            0.5, // Use medium temperature for aggregated solution
            base.token_count,
        );

        aggregated.phase = GenerationPhase::Aggregated;

        Ok(aggregated)
    }

    /// Combine reasoning from multiple solutions
    fn combine_reasoning(solutions: &[Solution]) -> String {
        let mut combined = String::from("Combined reasoning from multiple approaches:\n\n");

        for (idx, solution) in solutions.iter().enumerate() {
            combined.push_str(&format!("Approach {}:\n{}\n\n", idx + 1, solution.reasoning));
        }

        combined
    }

    /// Select the best answer from solutions
    ///
    /// Prefers answers that appear in multiple solutions (consensus)
    fn select_best_answer(solutions: &[Solution]) -> String {
        // Count answer frequency
        let mut answer_count: std::collections::HashMap<String, usize> = Default::default();

        for solution in solutions {
            *answer_count.entry(solution.answer.clone()).or_insert(0) += 1;
        }

        // Return most common answer, or first if no consensus
        answer_count
            .into_iter()
            .max_by_key(|(_answer, count)| *count)
            .map(|(answer, _count)| answer)
            .unwrap_or_else(|| {
                solutions
                    .first()
                    .map(|s| s.answer.clone())
                    .unwrap_or_default()
            })
    }

    /// Run Best-of-N sampling using any LLM provider
    ///
    /// This process:
    /// 1. Generates N diverse solutions with different temperatures
    /// 2. Evaluates each solution using the configured selection method
    /// 3. Returns the best solution based on the criteria
    /// 4. Simple but effective strategy for many use cases
    pub async fn aggregate_best_of_n(
        query: &str,
        system_prompt: &str,
        config: crate::best_of_n::BestOfNConfig,
        client: &dyn crate::ModelClient,
    ) -> Result<Vec<Solution>> {
        let (solution, _metadata) = crate::best_of_n::BestOfNAggregator::run_best_of_n(
            query,
            system_prompt,
            config,
            client,
        )
        .await?;

        Ok(vec![solution])
    }

    /// Run Self-Consistency using any LLM provider
    ///
    /// This process:
    /// 1. Generates K diverse reasoning paths
    /// 2. Extracts the final answer from each path
    /// 3. Uses consensus voting to select the best answer
    /// 4. Effective for tasks with multiple valid reasoning paths converging on correct answer
    pub async fn aggregate_self_consistency(
        query: &str,
        system_prompt: &str,
        config: crate::self_consistency::SelfConsistencyConfig,
        client: &dyn crate::ModelClient,
    ) -> Result<Vec<Solution>> {
        let (solution, _metadata) = crate::self_consistency::SelfConsistencyAggregator::run_self_consistency(
            query,
            system_prompt,
            config,
            client,
        )
        .await?;

        Ok(vec![solution])
    }

    /// Check if aggregation would benefit from more iterations
    pub fn should_continue_aggregation(
        solutions: &[Solution],
        current_iteration: usize,
        max_iterations: usize,
    ) -> bool {
        if current_iteration >= max_iterations {
            return false;
        }

        // Continue if we have no verified solutions yet
        let verified_count = solutions.iter().filter(|s| s.is_verified).count();
        verified_count == 0
    }

    /// Get aggregation statistics
    pub fn get_statistics(solutions: &[Solution]) -> AggregationStatistics {
        let total_solutions = solutions.len();
        let verified_solutions = solutions.iter().filter(|s| s.is_verified).count();
        let unverified_solutions = total_solutions - verified_solutions;

        let avg_score = if !solutions.is_empty() {
            solutions.iter().map(|s| s.verification_score).sum::<f32>() / total_solutions as f32
        } else {
            0.0
        };

        // Count unique answers
        let unique_answers: HashSet<_> =
            solutions.iter().map(|s| s.answer.clone()).collect();

        AggregationStatistics {
            total_solutions,
            verified_solutions,
            unverified_solutions,
            avg_verification_score: avg_score,
            unique_answers: unique_answers.len(),
        }
    }
}

/// Statistics about aggregation results
#[derive(Debug, Clone)]
pub struct AggregationStatistics {
    /// Total solutions in population
    pub total_solutions: usize,
    /// Number of verified solutions
    pub verified_solutions: usize,
    /// Number of unverified solutions
    pub unverified_solutions: usize,
    /// Average verification score
    pub avg_verification_score: f32,
    /// Number of unique answers produced
    pub unique_answers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_diverse_solutions() {
        let sol1 = Solution::new(
            "agent1".to_string(),
            "r1".to_string(),
            "a1".to_string(),
            0.3,
            100,
        );
        let sol2 = Solution::new(
            "agent2".to_string(),
            "r2".to_string(),
            "a2".to_string(),
            0.6,
            100,
        );
        let sol3 = Solution::new(
            "agent3".to_string(),
            "r3".to_string(),
            "a3".to_string(),
            1.0,
            100,
        );

        let solutions = vec![sol1, sol2, sol3];
        let selected = Aggregator::select_diverse_solutions(&solutions, 2).unwrap();
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_synthesize_solution() {
        let sol1 = Solution::new(
            "agent1".to_string(),
            "reasoning1".to_string(),
            "answer1".to_string(),
            0.3,
            100,
        );
        let sol2 = Solution::new(
            "agent2".to_string(),
            "reasoning2".to_string(),
            "answer1".to_string(),
            0.6,
            100,
        );

        let solutions = vec![sol1, sol2];
        let synthesized = Aggregator::synthesize_solution(&solutions, 0).unwrap();
        assert!(!synthesized.reasoning.is_empty());
        assert!(!synthesized.answer.is_empty());
        assert_eq!(synthesized.phase, GenerationPhase::Aggregated);
    }

    #[test]
    fn test_select_best_answer_consensus() {
        let sol1 = Solution::new(
            "agent1".to_string(),
            "r1".to_string(),
            "42".to_string(),
            0.3,
            100,
        );
        let sol2 = Solution::new(
            "agent2".to_string(),
            "r2".to_string(),
            "42".to_string(),
            0.6,
            100,
        );
        let sol3 = Solution::new(
            "agent3".to_string(),
            "r3".to_string(),
            "43".to_string(),
            1.0,
            100,
        );

        let solutions = vec![sol1, sol2, sol3];
        let best = Aggregator::select_best_answer(&solutions);
        assert_eq!(best, "42"); // Most common answer
    }

    #[test]
    fn test_aggregation_statistics() {
        let mut sol1 = Solution::new(
            "agent1".to_string(),
            "r1".to_string(),
            "a1".to_string(),
            0.3,
            100,
        );
        sol1.add_verification_pass(0.8);
        sol1.add_verification_pass(0.8);

        let sol2 = Solution::new(
            "agent2".to_string(),
            "r2".to_string(),
            "a2".to_string(),
            0.6,
            100,
        );

        let solutions = vec![sol1, sol2];
        let stats = Aggregator::get_statistics(&solutions);

        assert_eq!(stats.total_solutions, 2);
        assert_eq!(stats.verified_solutions, 1);
        assert_eq!(stats.unverified_solutions, 1);
    }
}
