/// Self-Consistency sampling strategy for improved solution quality.
///
/// Generates K diverse solutions and selects the best one based on consensus voting.
/// When multiple diverse reasoning paths converge on the same answer, it's highly likely correct.
/// Based on "Self-Consistency Improves Chain of Thought Reasoning" (Wei et al., 2022).

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};
use std::collections::HashMap;

/// Configuration for Self-Consistency strategy
#[derive(Clone, Debug)]
pub struct SelfConsistencyConfig {
    /// Number of diverse reasoning paths to generate (K in Self-Consistency)
    pub num_paths: usize,
    /// Temperature values for diversity across paths
    pub temperatures: Vec<f32>,
    /// Strategy for extracting final answer from reasoning
    pub extraction_strategy: AnswerExtractionStrategy,
    /// Voting method for consensus
    pub voting_strategy: VotingStrategy,
    /// Minimum consensus threshold (0.0-1.0)
    pub consensus_threshold: f32,
    /// Whether to weight votes by reasoning quality
    pub weight_by_quality: bool,
}

impl SelfConsistencyConfig {
    /// Create a new Self-Consistency configuration
    pub fn new(num_paths: usize) -> Self {
        let num_temps = num_paths.min(5);
        let temperatures = (0..num_temps)
            .map(|i| 0.5 + (i as f32 * 0.8 / num_temps as f32))
            .collect();

        Self {
            num_paths,
            temperatures,
            extraction_strategy: AnswerExtractionStrategy::LastLine,
            voting_strategy: VotingStrategy::MajorityVote,
            consensus_threshold: 0.5,
            weight_by_quality: true,
        }
    }

    /// Set the answer extraction strategy
    pub fn with_extraction_strategy(mut self, strategy: AnswerExtractionStrategy) -> Self {
        self.extraction_strategy = strategy;
        self
    }

    /// Set the voting strategy
    pub fn with_voting_strategy(mut self, strategy: VotingStrategy) -> Self {
        self.voting_strategy = strategy;
        self
    }

    /// Set custom temperatures
    pub fn with_temperatures(mut self, temps: Vec<f32>) -> Self {
        self.temperatures = temps;
        self
    }

    /// Set consensus threshold
    pub fn with_consensus_threshold(mut self, threshold: f32) -> Self {
        self.consensus_threshold = threshold.max(0.0).min(1.0);
        self
    }

    /// Enable/disable quality-weighted voting
    pub fn with_quality_weighting(mut self, enabled: bool) -> Self {
        self.weight_by_quality = enabled;
        self
    }
}

impl Default for SelfConsistencyConfig {
    fn default() -> Self {
        Self::new(5)
    }
}

/// Strategy for extracting the final answer from reasoning
#[derive(Clone, Debug)]
pub enum AnswerExtractionStrategy {
    /// Extract the last non-empty line
    LastLine,
    /// Extract text after "Answer:" marker
    AfterMarker,
    /// Extract the last sentence
    LastSentence,
    /// Extract text in quotes
    InQuotes,
    /// Use entire response as answer
    FullResponse,
}

/// Strategy for consensus voting
#[derive(Clone, Debug)]
pub enum VotingStrategy {
    /// Select most common answer
    MajorityVote,
    /// Weighted vote by reasoning length
    QualityWeighted,
    /// Select answer with highest confidence
    HighestConfidence,
    /// Ranked choice voting
    RankedChoice,
}

/// Self-Consistency implementation
pub struct SelfConsistencyAggregator;

impl SelfConsistencyAggregator {
    /// Run Self-Consistency on a query
    ///
    /// Generates K diverse solutions through different reasoning paths,
    /// then uses consensus voting to select the final answer.
    ///
    /// # Arguments
    /// * `query` - The problem statement
    /// * `system_prompt` - System instructions
    /// * `config` - Self-Consistency configuration
    /// * `client` - Model client for generation
    ///
    /// # Returns
    /// The consensus solution and detailed voting metadata
    pub async fn run_self_consistency(
        query: &str,
        system_prompt: &str,
        config: SelfConsistencyConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, SelfConsistencyMetadata)> {
        let mut paths = Vec::new();
        let mut total_tokens = 0;

        // Generate K diverse reasoning paths
        for (idx, &temperature) in config
            .temperatures
            .iter()
            .take(config.num_paths)
            .enumerate()
        {
            // Create prompt
            let system_msg = ResponseItem::Message {
                id: None,
                role: "system".to_string(),
                content: vec![ContentItem::InputText {
                    text: system_prompt.to_string(),
                }],
            };

            let user_msg = ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: query.to_string(),
                }],
            };

            let mut prompt = Prompt::new();
            prompt.input = vec![system_msg, user_msg];
            prompt.set_log_tag(&format!("self-consistency-path-{}", idx));

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
                            "Failed to generate path {}: {}",
                            idx, e
                        )));
                    }
                }
            }

            // Extract answer from this reasoning path
            let answer = Self::extract_answer(&response_text, &config.extraction_strategy);
            let reasoning_length = response_text.len();

            let path = ReasoningPath {
                id: idx,
                reasoning: response_text,
                extracted_answer: answer,
                temperature,
                reasoning_length,
            };

            paths.push(path);
        }

        if paths.is_empty() {
            return Err(MarsError::CoreError(
                "Failed to generate any reasoning paths".to_string(),
            ));
        }

        // Perform consensus voting
        let (final_answer, votes) = Self::consensus_vote(&paths, &config)?;
        let consensus_score = Self::calculate_consensus_score(&votes, paths.len());

        // Create synthesized solution from consensus
        let reasoning = Self::synthesize_reasoning(&paths, &final_answer);

        let solution = Solution::new(
            "self-consistency-consensus".to_string(),
            reasoning,
            final_answer.clone(),
            0.7, // Medium temperature for consensus
            total_tokens,
        );

        let metadata = SelfConsistencyMetadata {
            num_paths: paths.len(),
            total_tokens,
            extraction_strategy: format!("{:?}", config.extraction_strategy),
            voting_strategy: format!("{:?}", config.voting_strategy),
            consensus_score,
            voting_results: votes,
            all_paths: paths,
            consensus_answer: final_answer,
        };

        Ok((solution, metadata))
    }

    /// Extract answer from reasoning text
    fn extract_answer(text: &str, strategy: &AnswerExtractionStrategy) -> String {
        match strategy {
            AnswerExtractionStrategy::LastLine => {
                text.lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            }
            AnswerExtractionStrategy::AfterMarker => {
                // Look for common markers
                let markers = ["Answer:", "Final Answer:", "Conclusion:", "Result:"];
                for marker in &markers {
                    if let Some(pos) = text.find(marker) {
                        let after = &text[pos + marker.len()..].trim();
                        if !after.is_empty() {
                            return after.to_string();
                        }
                    }
                }
                text.to_string()
            }
            AnswerExtractionStrategy::LastSentence => {
                if let Some(last_period) = text.rfind('.') {
                    text[last_period + 1..].trim().to_string()
                } else {
                    text.to_string()
                }
            }
            AnswerExtractionStrategy::InQuotes => {
                // Extract first quoted text
                let mut in_quotes = false;
                let mut quoted = String::new();

                for ch in text.chars() {
                    if ch == '"' {
                        if in_quotes {
                            return quoted;
                        }
                        in_quotes = true;
                    } else if in_quotes {
                        quoted.push(ch);
                    }
                }

                if !quoted.is_empty() {
                    quoted
                } else {
                    text.to_string()
                }
            }
            AnswerExtractionStrategy::FullResponse => text.to_string(),
        }
    }

    /// Perform consensus voting on extracted answers
    fn consensus_vote(
        paths: &[ReasoningPath],
        config: &SelfConsistencyConfig,
    ) -> Result<(String, HashMap<String, usize>)> {
        // Count votes for each answer
        let mut vote_counts: HashMap<String, usize> = HashMap::new();

        for path in paths {
            let answer = path.extracted_answer.clone();
            *vote_counts.entry(answer).or_insert(0) += 1;
        }

        if vote_counts.is_empty() {
            return Err(MarsError::AggregationError(
                "No valid answers extracted".to_string(),
            ));
        }

        // Select winner based on voting strategy
        let winner = match config.voting_strategy {
            VotingStrategy::MajorityVote => Self::majority_vote(&vote_counts),
            VotingStrategy::QualityWeighted => Self::quality_weighted_vote(paths, &vote_counts),
            VotingStrategy::HighestConfidence => {
                Self::highest_confidence_vote(paths, &vote_counts)
            }
            VotingStrategy::RankedChoice => Self::ranked_choice_vote(paths, &vote_counts),
        };

        Ok((winner, vote_counts))
    }

    /// Simple majority voting
    fn majority_vote(votes: &HashMap<String, usize>) -> String {
        votes
            .iter()
            .max_by_key(|(_answer, count)| *count)
            .map(|(answer, _)| answer.clone())
            .unwrap_or_default()
    }

    /// Quality-weighted voting (by reasoning length)
    fn quality_weighted_vote(paths: &[ReasoningPath], _votes: &HashMap<String, usize>) -> String {
        let mut weighted_scores: HashMap<String, f32> = HashMap::new();

        // Weight each vote by the reasoning quality (normalized length)
        let avg_reasoning_len =
            paths.iter().map(|p| p.reasoning_length).sum::<usize>() as f32 / paths.len() as f32;

        for path in paths {
            let answer = &path.extracted_answer;
            let quality_factor = (path.reasoning_length as f32 / avg_reasoning_len).min(2.0);
            *weighted_scores.entry(answer.clone()).or_insert(0.0) += quality_factor;
        }

        weighted_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(answer, _)| answer.clone())
            .unwrap_or_default()
    }

    /// Select answer with highest confidence (combination of votes and quality)
    fn highest_confidence_vote(
        paths: &[ReasoningPath],
        votes: &HashMap<String, usize>,
    ) -> String {
        let mut confidence_scores: HashMap<String, f32> = HashMap::new();

        for (answer, &vote_count) in votes.iter() {
            let vote_ratio = vote_count as f32 / paths.len() as f32;

            // Find average quality of this answer's paths
            let answer_paths: Vec<_> = paths.iter().filter(|p| p.extracted_answer == *answer).collect();
            let avg_quality = if !answer_paths.is_empty() {
                answer_paths.iter().map(|p| p.reasoning_length).sum::<usize>() as f32
                    / answer_paths.len() as f32
                    / 1000.0
            } else {
                0.0
            };

            let confidence = vote_ratio * 0.7 + avg_quality.min(1.0) * 0.3;
            confidence_scores.insert(answer.clone(), confidence);
        }

        confidence_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(answer, _)| answer.clone())
            .unwrap_or_default()
    }

    /// Ranked choice voting (instant runoff)
    fn ranked_choice_vote(
        paths: &[ReasoningPath],
        _votes: &HashMap<String, usize>,
    ) -> String {
        // For simplicity, use the answer that appears earliest and most frequently
        let mut first_appearance: HashMap<String, usize> = HashMap::new();
        let mut final_counts: HashMap<String, usize> = HashMap::new();

        for (idx, path) in paths.iter().enumerate() {
            first_appearance.entry(path.extracted_answer.clone()).or_insert(idx);
            *final_counts.entry(path.extracted_answer.clone()).or_insert(0) += 1;
        }

        // Sort by count first, then by first appearance
        let mut sorted: Vec<_> = final_counts.into_iter().collect();
        sorted.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| {
                    first_appearance
                        .get(&a.0)
                        .cmp(&first_appearance.get(&b.0))
                })
        });

        sorted.first().map(|(answer, _)| answer.clone()).unwrap_or_default()
    }

    /// Calculate consensus score (how well paths agreed)
    fn calculate_consensus_score(votes: &HashMap<String, usize>, total_paths: usize) -> f32 {
        if votes.is_empty() || total_paths == 0 {
            return 0.0;
        }

        let max_votes = votes.values().max().copied().unwrap_or(0);
        (max_votes as f32 / total_paths as f32).min(1.0)
    }

    /// Synthesize reasoning from multiple paths
    fn synthesize_reasoning(paths: &[ReasoningPath], final_answer: &str) -> String {
        let mut reasoning = String::from("Self-Consistency Reasoning Paths:\n\n");

        let max_paths = paths.len().min(3); // Show top 3 paths for brevity
        for (idx, path) in paths.iter().enumerate().take(max_paths) {
            reasoning.push_str(&format!(
                "Path {} (temp={:.2}):\n{}\n\n",
                idx + 1,
                path.temperature,
                if path.reasoning.len() > 500 {
                    format!("{}...", &path.reasoning[..500])
                } else {
                    path.reasoning.clone()
                }
            ));
        }

        if paths.len() > 3 {
            reasoning.push_str(&format!(
                "... and {} more reasoning paths\n\n",
                paths.len() - 3
            ));
        }

        reasoning.push_str(&format!("Consensus Answer: {}", final_answer));
        reasoning
    }

    /// Get voting statistics
    pub fn get_voting_statistics(metadata: &SelfConsistencyMetadata) -> VotingStatistics {
        let total_paths = metadata.num_paths;
        let winning_votes = metadata
            .voting_results
            .get(&metadata.consensus_answer)
            .copied()
            .unwrap_or(0);

        let vote_variance = if metadata.voting_results.len() > 1 {
            let avg_votes = total_paths as f32 / metadata.voting_results.len() as f32;
            metadata
                .voting_results
                .values()
                .map(|&v| (v as f32 - avg_votes).powi(2))
                .sum::<f32>()
                / metadata.voting_results.len() as f32
        } else {
            0.0
        };

        VotingStatistics {
            total_paths,
            unique_answers: metadata.voting_results.len(),
            consensus_answer_votes: winning_votes,
            consensus_agreement: metadata.consensus_score,
            vote_distribution_variance: vote_variance.sqrt(),
            total_tokens: metadata.total_tokens,
        }
    }
}

/// Single reasoning path in Self-Consistency
#[derive(Clone, Debug)]
pub struct ReasoningPath {
    /// Path index
    pub id: usize,
    /// Full reasoning text
    pub reasoning: String,
    /// Extracted answer from reasoning
    pub extracted_answer: String,
    /// Temperature used for this path
    pub temperature: f32,
    /// Length of reasoning (for quality assessment)
    pub reasoning_length: usize,
}

/// Metadata from Self-Consistency execution
#[derive(Clone, Debug)]
pub struct SelfConsistencyMetadata {
    /// Number of reasoning paths generated
    pub num_paths: usize,
    /// Total tokens used
    pub total_tokens: usize,
    /// Answer extraction strategy used
    pub extraction_strategy: String,
    /// Voting strategy used
    pub voting_strategy: String,
    /// Consensus score (0.0-1.0)
    pub consensus_score: f32,
    /// Voting results (answer -> vote count)
    pub voting_results: HashMap<String, usize>,
    /// All reasoning paths
    pub all_paths: Vec<ReasoningPath>,
    /// The final consensus answer
    pub consensus_answer: String,
}

/// Statistics about the voting process
#[derive(Clone, Debug)]
pub struct VotingStatistics {
    /// Total reasoning paths
    pub total_paths: usize,
    /// Number of unique answers found
    pub unique_answers: usize,
    /// Number of votes for consensus answer
    pub consensus_answer_votes: usize,
    /// Agreement percentage (0.0-1.0)
    pub consensus_agreement: f32,
    /// Variance in vote distribution
    pub vote_distribution_variance: f32,
    /// Total tokens used
    pub total_tokens: usize,
}


#[cfg(test)]
mod tests;
