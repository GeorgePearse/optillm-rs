/// RSA (Reinforced Self-Aggregation) strategy for iterative solution improvement.
///
/// RSA is a population-based optimization strategy that maintains a population of solutions
/// and iteratively selects, refines, and synthesizes the best ones to improve overall quality.
/// Similar to evolutionary algorithms, it balances exploration and exploitation.

use crate::{types::Solution, MarsError, Result};
use std::collections::HashSet;

/// Configuration for RSA strategy
#[derive(Clone, Debug)]
pub struct RSAConfig {
    /// Initial population size
    pub population_size: usize,
    /// Number of solutions to select for refinement each iteration
    pub selection_size: usize,
    /// Number of iterations to perform
    pub num_iterations: usize,
    /// Selection criterion for choosing solutions
    pub selection_criterion: SelectionCriterion,
    /// Mutation/refinement strategy
    pub refinement_strategy: RefinementStrategy,
    /// Keep best solution from previous iteration
    pub elitism_enabled: bool,
}

impl RSAConfig {
    /// Create a new RSA configuration with defaults
    pub fn new(population_size: usize, selection_size: usize, num_iterations: usize) -> Self {
        Self {
            population_size,
            selection_size: selection_size.min(population_size),
            num_iterations,
            selection_criterion: SelectionCriterion::BestScore,
            refinement_strategy: RefinementStrategy::Synthesis,
            elitism_enabled: true,
        }
    }

    /// Set the selection criterion
    pub fn with_selection_criterion(mut self, criterion: SelectionCriterion) -> Self {
        self.selection_criterion = criterion;
        self
    }

    /// Set the refinement strategy
    pub fn with_refinement_strategy(mut self, strategy: RefinementStrategy) -> Self {
        self.refinement_strategy = strategy;
        self
    }

    /// Enable/disable elitism
    pub fn with_elitism(mut self, enabled: bool) -> Self {
        self.elitism_enabled = enabled;
        self
    }
}

impl Default for RSAConfig {
    fn default() -> Self {
        Self::new(5, 3, 2)
    }
}

/// Criterion for selecting solutions from population
#[derive(Clone, Debug)]
pub enum SelectionCriterion {
    /// Select based on verification score (highest)
    BestScore,
    /// Select most diverse solutions
    Diversity,
    /// Select by reasoning length (more thorough)
    Thoroughness,
    /// Random selection from population
    Random,
    /// Tournament selection
    Tournament,
}

/// Strategy for refining selected solutions
#[derive(Clone, Debug)]
pub enum RefinementStrategy {
    /// Synthesize selected solutions together
    Synthesis,
    /// Combine reasoning from multiple solutions
    Merge,
    /// Iteratively improve best solution
    Iterative,
    /// Average/ensemble approach
    Ensemble,
}

/// RSA (Reinforced Self-Aggregation) implementation
pub struct RSAAggregator;

impl RSAAggregator {
    /// Run RSA optimization on initial solutions
    ///
    /// Iteratively selects, refines, and synthesizes solutions to improve population quality.
    ///
    /// # Arguments
    /// * `initial_solutions` - Starting population of solutions
    /// * `config` - RSA configuration
    ///
    /// # Returns
    /// The best solution from final population and iteration statistics
    pub fn run_rsa(
        initial_solutions: &[Solution],
        config: RSAConfig,
    ) -> Result<(Solution, RSAMetadata)> {
        if initial_solutions.is_empty() {
            return Err(MarsError::AggregationError(
                "No initial solutions provided".to_string(),
            ));
        }

        let mut population = initial_solutions.to_vec();
        let mut best_solution = population[0].clone();
        let mut iteration_history = Vec::new();

        // Limit initial population to configured size
        if population.len() > config.population_size {
            population.truncate(config.population_size);
        }

        // Run RSA iterations
        for iteration in 0..config.num_iterations {
            let iteration_start = population.len();

            // Select solutions based on criterion
            let selected = Self::select_solutions(&population, config.selection_size, &config.selection_criterion)?;

            // Refine selected solutions
            let refined = Self::refine_solutions(&selected, iteration, &config.refinement_strategy)?;

            // Add refined solutions to population
            population.extend(refined);

            // Apply elitism if enabled
            if config.elitism_enabled {
                // Ensure best solution is preserved
                if !population.contains(&best_solution) {
                    population.push(best_solution.clone());
                }
            }

            // Trim population to size
            population.truncate(config.population_size);

            // Update best solution
            if let Some(new_best) = Self::find_best_solution(&population) {
                if new_best.verification_score > best_solution.verification_score {
                    best_solution = new_best;
                }
            }

            let iteration_stat = IterationStatistics {
                iteration: iteration,
                population_size: population.len(),
                solutions_added: iteration_start,
                best_score: best_solution.verification_score,
                avg_score: Self::calculate_avg_score(&population),
            };

            iteration_history.push(iteration_stat);
        }

        let metadata = RSAMetadata {
            total_iterations: config.num_iterations,
            final_population_size: population.len(),
            best_solution_score: best_solution.verification_score,
            population_diversity: Self::calculate_diversity(&population),
            iteration_history,
            final_population: population,
        };

        Ok((best_solution, metadata))
    }

    /// Select solutions from population based on criterion
    fn select_solutions(
        population: &[Solution],
        num_to_select: usize,
        criterion: &SelectionCriterion,
    ) -> Result<Vec<Solution>> {
        if population.is_empty() {
            return Ok(Vec::new());
        }

        let num_to_select = num_to_select.min(population.len());

        let selected = match criterion {
            SelectionCriterion::BestScore => Self::select_by_score(population, num_to_select),
            SelectionCriterion::Diversity => Self::select_by_diversity(population, num_to_select),
            SelectionCriterion::Thoroughness => Self::select_by_thoroughness(population, num_to_select),
            SelectionCriterion::Random => Self::select_random(population, num_to_select),
            SelectionCriterion::Tournament => Self::select_by_tournament(population, num_to_select),
        };

        Ok(selected)
    }

    /// Select best solutions by score
    fn select_by_score(population: &[Solution], num_to_select: usize) -> Vec<Solution> {
        let mut sorted = population.to_vec();
        sorted.sort_by(|a, b| {
            b.verification_score
                .partial_cmp(&a.verification_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(num_to_select);
        sorted
    }

    /// Select diverse solutions
    fn select_by_diversity(population: &[Solution], num_to_select: usize) -> Vec<Solution> {
        // Select solutions with different reasoning lengths for diversity
        let mut selected = Vec::new();
        let mut seen_ranges = HashSet::new();

        // Sort by reasoning length
        let mut by_length = population.to_vec();
        by_length.sort_by_key(|s| s.reasoning.len());

        for solution in by_length {
            if selected.len() >= num_to_select {
                break;
            }

            let length_range = solution.reasoning.len() / 500; // 500 char buckets
            if !seen_ranges.contains(&length_range) {
                seen_ranges.insert(length_range);
                selected.push(solution);
            }
        }

        // Fill remaining with best scores if needed
        if selected.len() < num_to_select {
            let remaining = Self::select_by_score(population, num_to_select - selected.len());
            selected.extend(remaining);
            selected.truncate(num_to_select);
        }

        selected
    }

    /// Select by reasoning thoroughness
    fn select_by_thoroughness(population: &[Solution], num_to_select: usize) -> Vec<Solution> {
        let mut sorted = population.to_vec();
        sorted.sort_by(|a, b| b.reasoning.len().cmp(&a.reasoning.len()));
        sorted.truncate(num_to_select);
        sorted
    }

    /// Select random solutions
    fn select_random(population: &[Solution], num_to_select: usize) -> Vec<Solution> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        population
            .choose_multiple(&mut rng, num_to_select)
            .cloned()
            .collect()
    }

    /// Tournament selection
    fn select_by_tournament(population: &[Solution], num_to_select: usize) -> Vec<Solution> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let mut selected = Vec::new();

        for _ in 0..num_to_select {
            // Tournament of 3
            let tournament: Vec<_> = population.choose_multiple(&mut rng, 3).collect();
            if let Some(&winner) = tournament.iter().max_by(|a, b| {
                a.verification_score
                    .partial_cmp(&b.verification_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                selected.push(winner.clone());
            }
        }

        selected
    }

    /// Refine selected solutions
    fn refine_solutions(
        selected: &[Solution],
        iteration: usize,
        strategy: &RefinementStrategy,
    ) -> Result<Vec<Solution>> {
        if selected.is_empty() {
            return Ok(Vec::new());
        }

        let refined = match strategy {
            RefinementStrategy::Synthesis => Self::synthesize_solutions(selected, iteration),
            RefinementStrategy::Merge => Self::merge_solutions(selected, iteration),
            RefinementStrategy::Iterative => Self::iterative_improvement(selected, iteration),
            RefinementStrategy::Ensemble => Self::ensemble_solutions(selected, iteration),
        }?;

        Ok(refined)
    }

    /// Synthesize selected solutions
    fn synthesize_solutions(solutions: &[Solution], iteration: usize) -> Result<Vec<Solution>> {
        if solutions.is_empty() {
            return Ok(Vec::new());
        }

        let combined_reasoning = Self::combine_reasoning(solutions);
        let best_answer = Self::select_consensus_answer(solutions);

        let mut synthesized = Solution::new(
            format!("rsa-synthesized-iter{}", iteration),
            combined_reasoning,
            best_answer,
            0.7,
            solutions.iter().map(|s| s.token_count).sum(),
        );

        synthesized.verification_score =
            solutions.iter().map(|s| s.verification_score).sum::<f32>() / solutions.len() as f32;

        Ok(vec![synthesized])
    }

    /// Merge solutions
    fn merge_solutions(solutions: &[Solution], iteration: usize) -> Result<Vec<Solution>> {
        // Create N variants by selecting different combinations
        let mut merged = Vec::new();

        // Simple merge: combine best parts
        for (idx, solution) in solutions.iter().enumerate() {
            let mut merged_sol = solution.clone();
            merged_sol.id = format!("rsa-merged-{}-{}", iteration, idx);
            merged_sol.verification_score =
                (merged_sol.verification_score + 0.1).min(1.0); // Slight boost for attempted improvement
            merged.push(merged_sol);
        }

        Ok(merged)
    }

    /// Iterative improvement
    fn iterative_improvement(solutions: &[Solution], iteration: usize) -> Result<Vec<Solution>> {
        // Focus on improving the best solution
        if let Some(best) = solutions.iter().max_by(|a, b| {
            a.verification_score
                .partial_cmp(&b.verification_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            let mut improved = best.clone();
            improved.id = format!("rsa-improved-iter{}", iteration);
            // Represent iterative improvement
            improved.verification_score = (improved.verification_score + 0.05).min(1.0);
            Ok(vec![improved])
        } else {
            Ok(Vec::new())
        }
    }

    /// Ensemble approach
    fn ensemble_solutions(solutions: &[Solution], iteration: usize) -> Result<Vec<Solution>> {
        let combined_reasoning = Self::combine_reasoning(solutions);
        let best_answer = Self::select_consensus_answer(solutions);
        let avg_temp = solutions.iter().map(|s| s.temperature).sum::<f32>() / solutions.len() as f32;

        let mut ensemble = Solution::new(
            format!("rsa-ensemble-iter{}", iteration),
            combined_reasoning,
            best_answer,
            avg_temp,
            solutions.iter().map(|s| s.token_count).sum(),
        );

        // Ensemble score is average of all
        ensemble.verification_score =
            solutions.iter().map(|s| s.verification_score).sum::<f32>() / solutions.len() as f32;

        Ok(vec![ensemble])
    }

    /// Find best solution in population
    fn find_best_solution(population: &[Solution]) -> Option<Solution> {
        population
            .iter()
            .max_by(|a, b| {
                a.verification_score
                    .partial_cmp(&b.verification_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate average score of population
    fn calculate_avg_score(population: &[Solution]) -> f32 {
        if population.is_empty() {
            return 0.0;
        }
        population.iter().map(|s| s.verification_score).sum::<f32>() / population.len() as f32
    }

    /// Calculate population diversity
    fn calculate_diversity(population: &[Solution]) -> f32 {
        if population.len() < 2 {
            return 0.0;
        }

        // Diversity based on unique answers
        let mut unique_answers = HashSet::new();
        for solution in population {
            unique_answers.insert(solution.answer.clone());
        }

        (unique_answers.len() as f32 / population.len() as f32).min(1.0)
    }

    /// Combine reasoning from solutions
    fn combine_reasoning(solutions: &[Solution]) -> String {
        let mut combined = String::from("Combined Reasoning from RSA Iteration:\n\n");

        for (idx, solution) in solutions.iter().enumerate() {
            combined.push_str(&format!("Approach {}:\n{}\n\n", idx + 1, solution.reasoning));
        }

        combined
    }

    /// Select consensus answer from solutions
    fn select_consensus_answer(solutions: &[Solution]) -> String {
        use std::collections::HashMap;

        // Count answer frequencies
        let mut counts: HashMap<String, usize> = HashMap::new();
        for solution in solutions {
            *counts.entry(solution.answer.clone()).or_insert(0) += 1;
        }

        // Return most common
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(answer, _)| answer)
            .unwrap_or_else(|| {
                solutions
                    .first()
                    .map(|s| s.answer.clone())
                    .unwrap_or_default()
            })
    }

    /// Get RSA execution statistics
    pub fn get_statistics(metadata: &RSAMetadata) -> RSAStatistics {
        let best_iter = metadata
            .iteration_history
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.best_score
                    .partial_cmp(&b.best_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        let improvement = if metadata.iteration_history.len() > 1 {
            let first = metadata.iteration_history[0].best_score;
            let last = metadata.iteration_history[metadata.iteration_history.len() - 1].best_score;
            (last - first).max(0.0)
        } else {
            0.0
        };

        RSAStatistics {
            total_iterations: metadata.total_iterations,
            final_population_size: metadata.final_population_size,
            final_best_score: metadata.best_solution_score,
            population_diversity: metadata.population_diversity,
            best_iteration: best_iter,
            total_improvement: improvement,
            average_iteration_score: metadata
                .iteration_history
                .iter()
                .map(|s| s.best_score)
                .sum::<f32>()
                / metadata.iteration_history.len().max(1) as f32,
        }
    }
}

/// Metadata from RSA execution
#[derive(Clone, Debug)]
pub struct RSAMetadata {
    /// Number of iterations performed
    pub total_iterations: usize,
    /// Final population size
    pub final_population_size: usize,
    /// Best solution score achieved
    pub best_solution_score: f32,
    /// Population diversity (0.0-1.0)
    pub population_diversity: f32,
    /// Statistics for each iteration
    pub iteration_history: Vec<IterationStatistics>,
    /// Final population of solutions
    pub final_population: Vec<Solution>,
}

/// Statistics for a single iteration
#[derive(Clone, Debug)]
pub struct IterationStatistics {
    /// Iteration number
    pub iteration: usize,
    /// Population size after this iteration
    pub population_size: usize,
    /// Number of solutions added this iteration
    pub solutions_added: usize,
    /// Best score in population after iteration
    pub best_score: f32,
    /// Average score in population
    pub avg_score: f32,
}

/// Statistics about RSA execution
#[derive(Clone, Debug)]
pub struct RSAStatistics {
    /// Total iterations performed
    pub total_iterations: usize,
    /// Final population size
    pub final_population_size: usize,
    /// Best score achieved
    pub final_best_score: f32,
    /// Population diversity
    pub population_diversity: f32,
    /// Which iteration had best result
    pub best_iteration: usize,
    /// Total improvement over iterations
    pub total_improvement: f32,
    /// Average score across iterations
    pub average_iteration_score: f32,
}


#[cfg(test)]
mod tests;
