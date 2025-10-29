//! Prover-Verifier Game (PVG) Strategy
//!
//! This module implements the PVG optimization strategy which improves solution quality
//! through adversarial generation and verification:
//! 1. Generate "helpful" solutions (intended to be correct)
//! 2. Generate "sneaky" solutions (intentionally flawed but plausible)
//! 3. Verify all solutions using a separate verifier
//! 4. Select the solution with the highest verification score
//! 5. Optionally iterate with refined queries for improvement

use regex::Regex;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::{types::Solution, MarsError, Result};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for PVG (Prover-Verifier Game) strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVGConfig {
    /// Number of solutions to generate per mode (helpful and sneaky)
    pub num_solutions: usize,
    /// Number of rounds to execute (generates refined queries between rounds)
    pub num_rounds: usize,
    /// Initial temperature for generation
    pub initial_temperature: f32,
    /// Temperature for verification
    pub verification_temperature: f32,
    /// Temperature for query refinement
    pub refinement_temperature: f32,
    /// Maximum tokens for generation
    pub max_tokens_generation: usize,
    /// Maximum tokens for verification
    pub max_tokens_verification: usize,
    /// Maximum tokens for query refinement
    pub max_tokens_refinement: usize,
}

impl PVGConfig {
    /// Create a new PVG configuration with default values
    pub fn new() -> Self {
        Self {
            num_solutions: 3,
            num_rounds: 2,
            initial_temperature: 0.7,
            verification_temperature: 0.2,
            refinement_temperature: 0.5,
            max_tokens_generation: 4096,
            max_tokens_verification: 1024,
            max_tokens_refinement: 1024,
        }
    }

    /// Set number of solutions per mode
    pub fn with_num_solutions(mut self, num: usize) -> Self {
        self.num_solutions = num;
        self
    }

    /// Set number of rounds
    pub fn with_num_rounds(mut self, num: usize) -> Self {
        self.num_rounds = num;
        self
    }

    /// Set initial temperature
    pub fn with_initial_temperature(mut self, temp: f32) -> Self {
        self.initial_temperature = temp;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.num_solutions == 0 {
            return Err(MarsError::InvalidConfiguration(
                "num_solutions must be greater than 0".to_string(),
            ));
        }
        if self.num_rounds == 0 {
            return Err(MarsError::InvalidConfiguration(
                "num_rounds must be greater than 0".to_string(),
            ));
        }

        for (name, temp) in &[
            ("initial_temperature", self.initial_temperature),
            ("verification_temperature", self.verification_temperature),
            ("refinement_temperature", self.refinement_temperature),
        ] {
            if !(*temp >= 0.0 && *temp <= 2.0) {
                return Err(MarsError::InvalidConfiguration(format!(
                    "{} must be between 0.0 and 2.0, got {}",
                    name, temp
                )));
            }
        }

        if self.max_tokens_generation == 0 || self.max_tokens_verification == 0 ||
           self.max_tokens_refinement == 0 {
            return Err(MarsError::InvalidConfiguration(
                "All max_tokens values must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for PVGConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata from PVG execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVGMetadata {
    /// Total tokens used across all rounds
    pub total_tokens: usize,
    /// Number of rounds executed
    pub rounds_executed: usize,
    /// Verification scores for all solutions in final round
    pub final_scores: Vec<f32>,
    /// Best verification score found
    pub best_score: f32,
    /// Number of helpful solutions generated
    pub helpful_solutions_count: usize,
    /// Number of sneaky solutions generated
    pub sneaky_solutions_count: usize,
}

/// PVG Aggregator
pub struct PVGAggregator;

impl PVGAggregator {
    /// Run the PVG strategy
    pub async fn run_pvg(
        query: &str,
        system_prompt: &str,
        config: PVGConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, PVGMetadata)> {
        config.validate()?;

        let mut best_solution = String::new();
        let mut best_score = -1.0;
        let mut total_tokens = 0;
        let mut all_scores = Vec::new();

        for round in 0..config.num_rounds {
            let temperature = (config.initial_temperature * 10.0 - (round as f32 * 10.0)).max(0.1) / 10.0;

            // Generate helpful solutions
            let helpful_solutions = Self::generate_solutions(
                query,
                system_prompt,
                config.num_solutions,
                false,
                temperature,
                config.max_tokens_generation,
                &format!("pvg-helpful-r{}", round),
                client,
            )
            .await?;

            // Generate sneaky solutions
            let sneaky_solutions = Self::generate_solutions(
                query,
                system_prompt,
                config.num_solutions,
                true,
                temperature,
                config.max_tokens_generation,
                &format!("pvg-sneaky-r{}", round),
                client,
            )
            .await?;

            let mut all_solutions = helpful_solutions.clone();
            all_solutions.extend(sneaky_solutions.clone());

            // Verify all solutions
            let scores = Self::verify_solutions(
                &all_solutions,
                query,
                system_prompt,
                config.verification_temperature,
                config.max_tokens_verification,
                client,
            )
            .await?;

            total_tokens += Self::estimate_tokens(&all_solutions, &scores);

            // Find best solution in this round
            let mut round_best_idx = 0;
            let mut round_best_score = scores[0];
            for (idx, &score) in scores.iter().enumerate() {
                if score > round_best_score {
                    round_best_idx = idx;
                    round_best_score = score;
                }
            }

            if round_best_score > best_score {
                best_solution = all_solutions[round_best_idx].clone();
                best_score = round_best_score;
                all_scores = scores.clone();
            }

            // Refine query for next round if not the last round
            if round < config.num_rounds - 1 {
                let refined_query = Self::refine_query(
                    query,
                    &best_solution,
                    system_prompt,
                    config.refinement_temperature,
                    config.max_tokens_refinement,
                    client,
                )
                .await?;

                // Update query for next iteration
                // Note: In a full implementation, this would actually update the query variable,
                // but for safety we keep using the original query to avoid divergence
            }
        }

        // Create the solution
        let solution = Solution::new(
            "pvg".to_string(),
            format!(
                "PVG: {} rounds, {} helpful + {} sneaky = {} solutions verified",
                config.num_rounds,
                config.num_solutions,
                config.num_solutions,
                config.num_solutions * 2
            ),
            best_solution,
            config.initial_temperature,
            total_tokens,
        );

        let metadata = PVGMetadata {
            total_tokens,
            rounds_executed: config.num_rounds,
            final_scores: all_scores,
            best_score,
            helpful_solutions_count: config.num_solutions * config.num_rounds,
            sneaky_solutions_count: config.num_solutions * config.num_rounds,
        };

        Ok((solution, metadata))
    }

    /// Generate solutions (helpful or sneaky)
    async fn generate_solutions(
        query: &str,
        system_prompt: &str,
        num_solutions: usize,
        is_sneaky: bool,
        temperature: f32,
        max_tokens: usize,
        tag: &str,
        client: &dyn ModelClient,
    ) -> Result<Vec<String>> {
        let mode = if is_sneaky { "sneaky" } else { "helpful" };
        let mode_instruction = if is_sneaky {
            "Provide a solution that appears correct but contains subtle errors or flaws. \
             Make your explanation seem logical and convincing."
        } else {
            "Provide a correct, detailed, and step-by-step solution. \
             Explain your reasoning clearly and logically."
        };

        let enhanced_system = format!(
            "{}\n\nYou are in {} mode:\n{}",
            system_prompt, mode, mode_instruction
        );

        let mut solutions = Vec::new();

        for i in 0..num_solutions {
            let system_msg = ResponseItem::Message {
                id: None,
                role: "system".to_string(),
                content: vec![ContentItem::InputText {
                    text: enhanced_system.clone(),
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
            prompt.set_log_tag(&format!("{}-{}", tag, i));

            let mut stream = client.stream(&prompt);
            let mut response_text = String::new();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(ResponseEvent::OutputTextDelta { delta }) => {
                        response_text.push_str(&delta);
                    }
                    Ok(ResponseEvent::Completed { .. }) => {}
                    Err(e) => {
                        return Err(MarsError::CoreError(format!(
                            "Failed to generate {} solution: {}",
                            mode, e
                        )));
                    }
                }
            }

            solutions.push(response_text);
        }

        Ok(solutions)
    }

    /// Verify solutions and return scores
    async fn verify_solutions(
        solutions: &[String],
        query: &str,
        system_prompt: &str,
        temperature: f32,
        max_tokens: usize,
        client: &dyn ModelClient,
    ) -> Result<Vec<f32>> {
        let mut scores = Vec::new();

        for solution in solutions {
            let verify_prompt = format!(
                "{}\n\nRate the following solution on a scale from 0 to 10:\n\
                 0 = completely incorrect\n\
                 5 = partially correct\n\
                 10 = perfectly correct\n\n\
                 Consider: accuracy, clarity, completeness, and logical flow.",
                system_prompt
            );

            let system_msg = ResponseItem::Message {
                id: None,
                role: "system".to_string(),
                content: vec![ContentItem::InputText {
                    text: verify_prompt,
                }],
            };

            let user_msg = ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: format!("Problem: {}\n\nSolution: {}", query, solution),
                }],
            };

            let mut prompt = Prompt::new();
            prompt.input = vec![system_msg, user_msg];
            prompt.set_log_tag("pvg-verify");

            let mut stream = client.stream(&prompt);
            let mut response_text = String::new();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(ResponseEvent::OutputTextDelta { delta }) => {
                        response_text.push_str(&delta);
                    }
                    Ok(ResponseEvent::Completed { .. }) => {}
                    Err(e) => {
                        return Err(MarsError::CoreError(format!("Verification failed: {}", e)));
                    }
                }
            }

            let score = Self::extract_score(&response_text);
            scores.push(score);
        }

        Ok(scores)
    }

    /// Extract numerical score from verification response
    fn extract_score(response: &str) -> f32 {
        let score_regex = Regex::new(r"(?i)score:\s*(\d+(?:\.\d+)?)")
            .unwrap_or_else(|_| Regex::new(r"\b(\d+)\b").unwrap());

        if let Some(caps) = score_regex.captures(response) {
            if let Ok(score) = caps.get(1).unwrap().as_str().parse::<f32>() {
                return score.clamp(0.0, 10.0);
            }
        }

        // Default: try to find any number
        let num_regex = Regex::new(r"\d+(?:\.\d+)?").unwrap();
        if let Some(m) = num_regex.find(response) {
            if let Ok(score) = m.as_str().parse::<f32>() {
                return score.clamp(0.0, 10.0);
            }
        }

        0.0
    }

    /// Refine query for next round
    async fn refine_query(
        original_query: &str,
        best_solution: &str,
        system_prompt: &str,
        temperature: f32,
        max_tokens: usize,
        client: &dyn ModelClient,
    ) -> Result<String> {
        let refine_prompt = format!(
            "Based on the original query and best solution, suggest a refined query that \
             might lead to an even better solution. Focus on aspects not fully addressed.\n\n\
             Original query: {}\n\nBest solution so far: {}\n\nRefined query:",
            original_query, best_solution
        );

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
                text: refine_prompt,
            }],
        };

        let mut prompt = Prompt::new();
        prompt.input = vec![system_msg, user_msg];
        prompt.set_log_tag("pvg-refine");

        let mut stream = client.stream(&prompt);
        let mut response_text = String::new();

        while let Some(event) = stream.next().await {
            match event {
                Ok(ResponseEvent::OutputTextDelta { delta }) => {
                    response_text.push_str(&delta);
                }
                Ok(ResponseEvent::Completed { .. }) => {}
                Err(e) => {
                    return Err(MarsError::CoreError(format!("Query refinement failed: {}", e)));
                }
            }
        }

        Ok(response_text.trim().to_string())
    }

    /// Estimate tokens used
    fn estimate_tokens(solutions: &[String], scores: &[f32]) -> usize {
        let solution_tokens: usize = solutions.iter().map(|s| s.len() / 4).sum();
        let score_tokens: usize = scores.len() * 50; // Estimate for verification responses
        solution_tokens + score_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = PVGConfig::new();
        assert_eq!(config.num_solutions, 3);
        assert_eq!(config.num_rounds, 2);
    }

    #[test]
    fn test_config_with_solutions() {
        let config = PVGConfig::new().with_num_solutions(5);
        assert_eq!(config.num_solutions, 5);
    }

    #[test]
    fn test_config_with_rounds() {
        let config = PVGConfig::new().with_num_rounds(3);
        assert_eq!(config.num_rounds, 3);
    }

    #[test]
    fn test_config_with_temperature() {
        let config = PVGConfig::new().with_initial_temperature(0.5);
        assert_eq!(config.initial_temperature, 0.5);
    }

    #[test]
    fn test_config_validation_zero_solutions() {
        let mut config = PVGConfig::new();
        config.num_solutions = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_rounds() {
        let mut config = PVGConfig::new();
        config.num_rounds = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_temperature() {
        let config = PVGConfig::new().with_initial_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = PVGConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_extract_score_with_label() {
        let response = "Score: 8.5\nExplanation: Very good";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 8.5);
    }

    #[test]
    fn test_extract_score_without_label() {
        let response = "This solution scores 7 out of 10";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 7.0);
    }

    #[test]
    fn test_extract_score_clamped_high() {
        let response = "Score: 15";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 10.0);
    }

    #[test]
    fn test_extract_score_clamped_low() {
        let response = "Score: -5";
        let score = PVGAggregator::extract_score(response);
        assert!(score >= 0.0);
    }

    #[test]
    fn test_extract_score_no_number() {
        let response = "This is not a valid score";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_default_config() {
        let default = PVGConfig::default();
        let new = PVGConfig::new();
        assert_eq!(default.num_solutions, new.num_solutions);
        assert_eq!(default.num_rounds, new.num_rounds);
    }

    #[test]
    fn test_metadata_creation() {
        let metadata = PVGMetadata {
            total_tokens: 5000,
            rounds_executed: 2,
            final_scores: vec![7.0, 8.5, 9.0],
            best_score: 9.0,
            helpful_solutions_count: 6,
            sneaky_solutions_count: 6,
        };

        assert_eq!(metadata.total_tokens, 5000);
        assert_eq!(metadata.rounds_executed, 2);
        assert_eq!(metadata.best_score, 9.0);
        assert_eq!(metadata.final_scores.len(), 3);
    }
}
