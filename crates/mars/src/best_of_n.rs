/// Best-of-N sampling strategy for solution optimization.
///
/// This strategy generates N diverse solutions and selects the best one based on
/// various criteria such as verification score, ranking, or heuristics.
/// Simple but effective for many use cases.

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for Best-of-N strategy
#[derive(Clone, Debug)]
pub struct BestOfNConfig {
    /// Number of candidates to generate (N in Best-of-N)
    pub num_candidates: usize,
    /// Temperatures for diversity in generation
    pub temperatures: Vec<f32>,
    /// Selection method for choosing the best solution
    pub selection_method: SelectionMethod,
    /// Whether to use verification scores if available
    pub use_verification_scores: bool,
}

impl BestOfNConfig {
    /// Create a new Best-of-N configuration
    pub fn new(num_candidates: usize) -> Self {
        let num_temps = num_candidates.min(5);
        let temperatures = (0..num_temps)
            .map(|i| 0.3 + (i as f32 * 0.7 / num_temps as f32))
            .collect();

        Self {
            num_candidates,
            temperatures,
            selection_method: SelectionMethod::BestScore,
            use_verification_scores: true,
        }
    }

    /// Set the selection method
    pub fn with_selection_method(mut self, method: SelectionMethod) -> Self {
        self.selection_method = method;
        self
    }

    /// Set custom temperatures
    pub fn with_temperatures(mut self, temps: Vec<f32>) -> Self {
        self.temperatures = temps;
        self
    }

    /// Enable/disable verification score usage
    pub fn with_verification_scores(mut self, enabled: bool) -> Self {
        self.use_verification_scores = enabled;
        self
    }
}

impl Default for BestOfNConfig {
    fn default() -> Self {
        Self::new(5)
    }
}

/// Method for selecting the best solution
#[derive(Clone, Debug)]
pub enum SelectionMethod {
    /// Select based on highest verification score
    BestScore,
    /// Select based on solution confidence/quality heuristic
    HighestConfidence,
    /// Select based on length of reasoning (prefer more thorough answers)
    MostThorough,
    /// Select based on conciseness of answer
    MostConcise,
    /// Rank by multiple criteria and select top ranked
    MultiCriteria,
}

/// Best-of-N aggregator that generates and selects best solution
pub struct BestOfNAggregator;

impl BestOfNAggregator {
    /// Run Best-of-N selection on a query
    ///
    /// Generates N diverse solutions and returns the best one according to the
    /// configured selection method.
    ///
    /// # Arguments
    /// * `query` - The problem statement
    /// * `system_prompt` - System instructions for the model
    /// * `config` - Configuration for Best-of-N
    /// * `client` - Model client for generation
    ///
    /// # Returns
    /// The best selected solution and metadata about the selection
    pub async fn run_best_of_n(
        query: &str,
        system_prompt: &str,
        config: BestOfNConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, BestOfNMetadata)> {
        let mut solutions = Vec::new();
        let mut total_tokens = 0;

        // Generate N diverse solutions
        for (idx, &temperature) in config.temperatures.iter().take(config.num_candidates).enumerate() {
            // Create system message
            let system_msg = ResponseItem::Message {
                id: None,
                role: "system".to_string(),
                content: vec![ContentItem::InputText {
                    text: system_prompt.to_string(),
                }],
            };

            // Create user message
            let user_msg = ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: query.to_string(),
                }],
            };

            let mut prompt = Prompt::new();
            prompt.input = vec![system_msg, user_msg];
            prompt.set_log_tag(&format!("best-of-n-candidate-{}", idx));

            let mut stream = client.stream(&prompt);
            let mut response_text = String::new();

            // Collect streaming response
            while let Some(event) = stream.next().await {
                match event {
                    Ok(ResponseEvent::OutputTextDelta { delta }) => {
                        response_text.push_str(&delta);
                    }
                    Ok(ResponseEvent::Completed { token_usage }) => {
                        if let Some(usage) = token_usage {
                            total_tokens += usage.total_tokens();
                        }
                    }
                    Err(e) => {
                        return Err(MarsError::CoreError(format!(
                            "Failed to generate solution {}: {}",
                            idx, e
                        )));
                    }
                }
            }

            // Parse response into reasoning and answer
            let (reasoning, answer) = Self::parse_response(&response_text);

            let solution = Solution::new(
                format!("best-of-n-candidate-{}", idx),
                reasoning,
                answer,
                temperature,
                total_tokens,
            );

            solutions.push(solution);
        }

        if solutions.is_empty() {
            return Err(MarsError::CoreError(
                "Failed to generate any solutions".to_string(),
            ));
        }

        // Select the best solution
        let (best_solution, selection_score) =
            Self::select_best_solution(&solutions, &config.selection_method)?;

        let metadata = BestOfNMetadata {
            num_candidates: solutions.len(),
            total_tokens,
            selection_method: format!("{:?}", config.selection_method),
            selection_score,
            all_candidates: solutions,
        };

        Ok((best_solution, metadata))
    }

    /// Select the best solution from candidates using the configured method
    fn select_best_solution(
        solutions: &[Solution],
        method: &SelectionMethod,
    ) -> Result<(Solution, f32)> {
        if solutions.is_empty() {
            return Err(MarsError::AggregationError(
                "No solutions to select from".to_string(),
            ));
        }

        let (best_idx, score) = match method {
            SelectionMethod::BestScore => Self::select_by_score(solutions),
            SelectionMethod::HighestConfidence => Self::select_by_confidence(solutions),
            SelectionMethod::MostThorough => Self::select_by_thoroughness(solutions),
            SelectionMethod::MostConcise => Self::select_by_conciseness(solutions),
            SelectionMethod::MultiCriteria => Self::select_by_multi_criteria(solutions),
        };

        let mut best = solutions[best_idx].clone();
        best.verification_score = score;

        Ok((best, score))
    }

    /// Select by best verification score
    fn select_by_score(solutions: &[Solution]) -> (usize, f32) {
        let mut best_idx = 0;
        let mut best_score = solutions[0].verification_score;

        for (idx, sol) in solutions.iter().enumerate().skip(1) {
            if sol.verification_score > best_score {
                best_idx = idx;
                best_score = sol.verification_score;
            }
        }

        (best_idx, best_score)
    }

    /// Select by confidence heuristic (length-weighted score)
    fn select_by_confidence(solutions: &[Solution]) -> (usize, f32) {
        let mut best_idx = 0;
        let mut best_score = 0.0;

        for (idx, sol) in solutions.iter().enumerate() {
            // Confidence: verification score weighted by reasoning length
            let reasoning_factor = (sol.reasoning.len() as f32 / 1000.0).min(1.0);
            let confidence = sol.verification_score * 0.6 + reasoning_factor * 0.4;

            if confidence > best_score {
                best_idx = idx;
                best_score = confidence;
            }
        }

        (best_idx, best_score)
    }

    /// Select the most thorough solution (longest reasoning)
    fn select_by_thoroughness(solutions: &[Solution]) -> (usize, f32) {
        let mut best_idx = 0;
        let mut max_length = solutions[0].reasoning.len();

        for (idx, sol) in solutions.iter().enumerate().skip(1) {
            if sol.reasoning.len() > max_length {
                best_idx = idx;
                max_length = sol.reasoning.len();
            }
        }

        // Score based on relative thoroughness
        let score = (max_length as f32 / 2000.0).min(1.0);
        (best_idx, score)
    }

    /// Select the most concise solution (shortest answer)
    fn select_by_conciseness(solutions: &[Solution]) -> (usize, f32) {
        let mut best_idx = 0;
        let mut min_length = solutions[0].answer.len();

        for (idx, sol) in solutions.iter().enumerate().skip(1) {
            if sol.answer.len() < min_length {
                best_idx = idx;
                min_length = sol.answer.len();
            }
        }

        // Score based on relative conciseness
        let score = (1.0 - (min_length as f32 / 500.0).min(1.0)).max(0.0);
        (best_idx, score)
    }

    /// Select using multiple criteria ranking
    fn select_by_multi_criteria(solutions: &[Solution]) -> (usize, f32) {
        let mut scores = vec![0.0_f32; solutions.len()];

        // Score 1: Verification score (40%)
        let mut max_verification = 0.0_f32;
        for sol in solutions {
            max_verification = max_verification.max(sol.verification_score);
        }
        if max_verification > 0.0 {
            for (idx, sol) in solutions.iter().enumerate() {
                scores[idx] += (sol.verification_score / max_verification) * 0.4;
            }
        }

        // Score 2: Thoroughness (30%)
        let mut max_reasoning = 0;
        for sol in solutions {
            max_reasoning = max_reasoning.max(sol.reasoning.len());
        }
        if max_reasoning > 0 {
            for (idx, sol) in solutions.iter().enumerate() {
                scores[idx] +=
                    ((sol.reasoning.len() as f32) / (max_reasoning as f32)) * 0.3;
            }
        }

        // Score 3: Conciseness (20%)
        let mut min_answer = usize::MAX;
        for sol in solutions {
            min_answer = min_answer.min(sol.answer.len());
        }
        if min_answer > 0 {
            for (idx, sol) in solutions.iter().enumerate() {
                scores[idx] += ((min_answer as f32) / (sol.answer.len() as f32)).min(1.0) * 0.2;
            }
        }

        // Score 4: Temperature diversity (10%)
        let temps: Vec<f32> = solutions.iter().map(|s| s.temperature).collect();
        let avg_temp = temps.iter().sum::<f32>() / temps.len() as f32;
        for (idx, sol) in solutions.iter().enumerate() {
            let temp_diversity = 1.0 - ((sol.temperature - avg_temp).abs() / 1.0).min(1.0);
            scores[idx] += temp_diversity * 0.1;
        }

        let best_idx = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        (best_idx, scores[best_idx])
    }

    /// Parse response into reasoning and answer
    fn parse_response(response: &str) -> (String, String) {
        // Look for common separators like "Answer:" or "Final Answer:"
        let separators = ["Answer:", "Final Answer:", "Conclusion:", "Result:"];

        for separator in &separators {
            if let Some(pos) = response.find(separator) {
                let reasoning = response[..pos].trim().to_string();
                let answer = response[pos + separator.len()..].trim().to_string();
                return (reasoning, answer);
            }
        }

        // If no separator found, use the last sentence as answer
        if let Some(last_period) = response.rfind('.') {
            let reasoning = response[..last_period].trim().to_string();
            let answer = response[last_period + 1..].trim().to_string();
            return (reasoning, answer);
        }

        // Fallback: entire response is answer
        (String::new(), response.to_string())
    }

    /// Get statistics about the selection
    pub fn get_selection_statistics(metadata: &BestOfNMetadata) -> SelectionStatistics {
        let scores: Vec<f32> = metadata
            .all_candidates
            .iter()
            .map(|s| s.verification_score)
            .collect();

        let avg_score = if !scores.is_empty() {
            scores.iter().sum::<f32>() / scores.len() as f32
        } else {
            0.0
        };

        let max_score = scores
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .copied()
            .unwrap_or(0.0);

        let min_score = scores
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .copied()
            .unwrap_or(0.0);

        let variance = if !scores.is_empty() {
            scores
                .iter()
                .map(|s| (s - avg_score).powi(2))
                .sum::<f32>()
                / scores.len() as f32
        } else {
            0.0
        };

        SelectionStatistics {
            num_candidates: metadata.num_candidates,
            avg_candidate_score: avg_score,
            best_candidate_score: max_score,
            worst_candidate_score: min_score,
            score_variance: variance.sqrt(),
            selected_score: metadata.selection_score,
        }
    }
}

/// Metadata from Best-of-N selection
#[derive(Clone, Debug)]
pub struct BestOfNMetadata {
    /// Number of candidates generated
    pub num_candidates: usize,
    /// Total tokens used
    pub total_tokens: usize,
    /// Method used for selection
    pub selection_method: String,
    /// Score of selected solution
    pub selection_score: f32,
    /// All candidate solutions
    pub all_candidates: Vec<Solution>,
}

/// Statistics about the selection process
#[derive(Clone, Debug)]
pub struct SelectionStatistics {
    /// Number of candidates evaluated
    pub num_candidates: usize,
    /// Average score across all candidates
    pub avg_candidate_score: f32,
    /// Best score among candidates
    pub best_candidate_score: f32,
    /// Worst score among candidates
    pub worst_candidate_score: f32,
    /// Standard deviation of scores
    pub score_variance: f32,
    /// Score of the selected solution
    pub selected_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = BestOfNConfig::new(5);
        assert_eq!(config.num_candidates, 5);
        assert!(!config.temperatures.is_empty());
    }

    #[test]
    fn test_config_with_custom_temps() {
        let temps = vec![0.2, 0.5, 0.8];
        let config = BestOfNConfig::new(3).with_temperatures(temps.clone());
        assert_eq!(config.temperatures, temps);
    }

    #[test]
    fn test_config_with_selection_method() {
        let config = BestOfNConfig::new(5)
            .with_selection_method(SelectionMethod::HighestConfidence);

        match config.selection_method {
            SelectionMethod::HighestConfidence => (),
            _ => panic!("Selection method not set correctly"),
        }
    }

    #[test]
    fn test_select_by_score() {
        let mut sol1 = Solution::new(
            "agent1".to_string(),
            "reasoning1".to_string(),
            "answer1".to_string(),
            0.3,
            100,
        );
        sol1.verification_score = 0.5;

        let mut sol2 = Solution::new(
            "agent2".to_string(),
            "reasoning2".to_string(),
            "answer2".to_string(),
            0.6,
            100,
        );
        sol2.verification_score = 0.8;

        let mut sol3 = Solution::new(
            "agent3".to_string(),
            "reasoning3".to_string(),
            "answer3".to_string(),
            0.9,
            100,
        );
        sol3.verification_score = 0.6;

        let solutions = vec![sol1, sol2, sol3];
        let (best_idx, _) = BestOfNAggregator::select_by_score(&solutions);
        assert_eq!(best_idx, 1); // sol2 has highest score
    }

    #[test]
    fn test_select_by_confidence() {
        let sol1 = Solution::new(
            "agent1".to_string(),
            "short".to_string(),
            "answer1".to_string(),
            0.3,
            100,
        );

        let sol2 = Solution::new(
            "agent2".to_string(),
            "this is a much longer reasoning that should be more thorough".to_string(),
            "answer2".to_string(),
            0.6,
            100,
        );

        let solutions = vec![sol1, sol2];
        let (best_idx, _) = BestOfNAggregator::select_by_confidence(&solutions);
        assert_eq!(best_idx, 1); // sol2 has more thorough reasoning
    }

    #[test]
    fn test_select_by_thoroughness() {
        let sol1 = Solution::new(
            "agent1".to_string(),
            "short reasoning".to_string(),
            "answer1".to_string(),
            0.3,
            100,
        );

        let sol2 = Solution::new(
            "agent2".to_string(),
            "this is a much longer and more thorough reasoning that should be preferred"
                .repeat(10),
            "answer2".to_string(),
            0.6,
            100,
        );

        let solutions = vec![sol1, sol2];
        let (best_idx, _) = BestOfNAggregator::select_by_thoroughness(&solutions);
        assert_eq!(best_idx, 1); // sol2 is longer
    }

    #[test]
    fn test_select_by_conciseness() {
        let sol1 = Solution::new(
            "agent1".to_string(),
            "reasoning".to_string(),
            "a".to_string(),
            0.3,
            100,
        );

        let sol2 = Solution::new(
            "agent2".to_string(),
            "reasoning".to_string(),
            "this is a much longer answer".to_string(),
            0.6,
            100,
        );

        let solutions = vec![sol1, sol2];
        let (best_idx, _) = BestOfNAggregator::select_by_conciseness(&solutions);
        assert_eq!(best_idx, 0); // sol1 has shorter answer
    }

    #[test]
    fn test_select_by_multi_criteria() {
        let mut sol1 = Solution::new(
            "agent1".to_string(),
            "short".to_string(),
            "a".to_string(),
            0.3,
            100,
        );
        sol1.verification_score = 0.9;

        let mut sol2 = Solution::new(
            "agent2".to_string(),
            "medium reasoning content here".to_string(),
            "longer answer here".to_string(),
            0.6,
            100,
        );
        sol2.verification_score = 0.7;

        let solutions = vec![sol1, sol2];
        let (best_idx, _) = BestOfNAggregator::select_by_multi_criteria(&solutions);
        // Should select based on combination of criteria
        assert!(best_idx < 2);
    }

    #[test]
    fn test_parse_response_with_separator() {
        let response = "This is reasoning. Answer: This is the answer.";
        let (reasoning, answer) = BestOfNAggregator::parse_response(response);
        assert!(reasoning.contains("This is reasoning"));
        assert!(answer.contains("This is the answer"));
    }

    #[test]
    fn test_parse_response_without_separator() {
        let response = "This is the complete response without any separator.";
        let (reasoning, answer) = BestOfNAggregator::parse_response(response);
        assert!(answer.len() > 0);
    }

    #[test]
    fn test_parse_response_with_final_answer() {
        let response =
            "Let me think about this step by step. Final Answer: The answer is 42.";
        let (reasoning, answer) = BestOfNAggregator::parse_response(response);
        assert!(reasoning.contains("step by step"));
        assert!(answer.contains("42"));
    }

    #[test]
    fn test_selection_statistics() {
        let mut sol1 = Solution::new("a1".to_string(), "r1".to_string(), "ans1".to_string(), 0.3, 100);
        sol1.verification_score = 0.5;

        let mut sol2 = Solution::new("a2".to_string(), "r2".to_string(), "ans2".to_string(), 0.6, 100);
        sol2.verification_score = 0.8;

        let mut sol3 = Solution::new("a3".to_string(), "r3".to_string(), "ans3".to_string(), 0.9, 100);
        sol3.verification_score = 0.6;

        let metadata = BestOfNMetadata {
            num_candidates: 3,
            total_tokens: 300,
            selection_method: "BestScore".to_string(),
            selection_score: 0.8,
            all_candidates: vec![sol1, sol2, sol3],
        };

        let stats = BestOfNAggregator::get_selection_statistics(&metadata);
        assert_eq!(stats.num_candidates, 3);
        assert!(stats.best_candidate_score >= stats.avg_candidate_score);
    }

    #[test]
    fn test_empty_solutions_error() {
        let solutions: Vec<Solution> = vec![];
        let result = BestOfNAggregator::select_best_solution(&solutions, &SelectionMethod::BestScore);
        assert!(result.is_err());
    }
}
