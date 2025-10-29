//! Round Trip Optimization (RTO) Strategy
//!
//! This module implements the RTO optimization strategy which improves code quality through
//! a round-trip generation process:
//! 1. Generate initial solution (C1)
//! 2. Ask model to describe/summarize the solution as instructions (Q2)
//! 3. Generate new solution from description (C2)
//! 4. If C1 and C2 differ, synthesize them into final solution (C3)
//! 5. Return the best solution

use regex::Regex;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::{types::Solution, MarsError, Result};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for RTO (Round Trip Optimization) strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RTOConfig {
    /// Temperature for initial generation (C1)
    pub initial_temperature: f32,
    /// Temperature for description generation (Q2)
    pub description_temperature: f32,
    /// Temperature for second generation (C2)
    pub second_temperature: f32,
    /// Temperature for synthesis (C3) if C1 != C2
    pub synthesis_temperature: f32,
    /// Maximum tokens for initial generation
    pub max_tokens_initial: usize,
    /// Maximum tokens for description
    pub max_tokens_description: usize,
    /// Maximum tokens for second generation
    pub max_tokens_second: usize,
    /// Maximum tokens for synthesis
    pub max_tokens_synthesis: usize,
}

impl RTOConfig {
    /// Create a new RTO configuration with default values
    pub fn new() -> Self {
        Self {
            initial_temperature: 0.1,
            description_temperature: 0.1,
            second_temperature: 0.1,
            synthesis_temperature: 0.1,
            max_tokens_initial: 4096,
            max_tokens_description: 1024,
            max_tokens_second: 4096,
            max_tokens_synthesis: 4096,
        }
    }

    /// Set initial generation temperature
    pub fn with_initial_temperature(mut self, temp: f32) -> Self {
        self.initial_temperature = temp;
        self
    }

    /// Set all temperatures at once
    pub fn with_all_temperatures(mut self, temp: f32) -> Self {
        self.initial_temperature = temp;
        self.description_temperature = temp;
        self.second_temperature = temp;
        self.synthesis_temperature = temp;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        for (name, temp) in &[
            ("initial_temperature", self.initial_temperature),
            ("description_temperature", self.description_temperature),
            ("second_temperature", self.second_temperature),
            ("synthesis_temperature", self.synthesis_temperature),
        ] {
            if !(*temp >= 0.0 && *temp <= 2.0) {
                return Err(MarsError::InvalidConfiguration(format!(
                    "{} must be between 0.0 and 2.0, got {}",
                    name, temp
                )));
            }
        }

        if self.max_tokens_initial == 0 || self.max_tokens_description == 0 ||
           self.max_tokens_second == 0 || self.max_tokens_synthesis == 0 {
            return Err(MarsError::InvalidConfiguration(
                "All max_tokens values must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for RTOConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata from RTO execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RTOMetadata {
    /// Total tokens used across all rounds
    pub total_tokens: usize,
    /// Initial solution (C1)
    pub initial_solution: String,
    /// Description of initial solution (Q2)
    pub description: String,
    /// Second solution (C2)
    pub second_solution: String,
    /// Whether C1 and C2 differed
    pub solutions_differed: bool,
    /// Final synthesized solution (C3) if applicable
    pub synthesized_solution: Option<String>,
}

/// RTO Aggregator
pub struct RTOAggregator;

impl RTOAggregator {
    /// Run the RTO strategy
    pub async fn run_rto(
        query: &str,
        system_prompt: &str,
        config: RTOConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, RTOMetadata)> {
        config.validate()?;

        // Step 1: Generate initial solution (C1)
        let (initial_solution, tokens_c1) = Self::generate_solution(
            query,
            system_prompt,
            config.initial_temperature,
            config.max_tokens_initial,
            "rto-c1",
            client,
        )
        .await?;

        // Step 2: Ask for description (Q2)
        let description_prompt = format!(
            "Summarize or describe the following solution. \
             The summary should be in the form of an instruction such that, \
             given the instruction you can create the solution yourself.\n\n\
             Solution:\n{}",
            initial_solution
        );

        let (description, tokens_q2) = Self::generate_solution(
            &description_prompt,
            system_prompt,
            config.description_temperature,
            config.max_tokens_description,
            "rto-q2",
            client,
        )
        .await?;

        // Step 3: Generate second solution from description (C2)
        let (second_solution, tokens_c2) = Self::generate_solution(
            &description,
            system_prompt,
            config.second_temperature,
            config.max_tokens_second,
            "rto-c2",
            client,
        )
        .await?;

        // Step 4: Check if solutions differ
        let solutions_differed = Self::solutions_differ(&initial_solution, &second_solution);

        // Step 5: Synthesize if different
        let (final_answer, tokens_c3, synthesized) = if solutions_differed {
            let synthesis_prompt = format!(
                "Based on the original query and these two different implementations, \
                 generate a final, optimized version.\n\n\
                 Original query: {}\n\n\
                 First solution (C1):\n{}\n\n\
                 Second solution (C2):\n{}\n\n\
                 Provide only the final optimized solution without explanations.",
                query, initial_solution, second_solution
            );

            let (synthesized_sol, tokens) = Self::generate_solution(
                &synthesis_prompt,
                system_prompt,
                config.synthesis_temperature,
                config.max_tokens_synthesis,
                "rto-c3",
                client,
            )
            .await?;

            (synthesized_sol.clone(), tokens, Some(synthesized_sol))
        } else {
            (initial_solution.clone(), 0, None)
        };

        let total_tokens = tokens_c1 + tokens_q2 + tokens_c2 + tokens_c3;

        // Create the solution
        let solution = Solution::new(
            "rto".to_string(),
            format!("RTO: C1 -> Q2 -> C2 -> {}", if solutions_differed { "C3" } else { "Final" }),
            final_answer,
            config.initial_temperature,
            total_tokens,
        );

        // Create metadata
        let metadata = RTOMetadata {
            total_tokens,
            initial_solution,
            description,
            second_solution,
            solutions_differed,
            synthesized_solution: synthesized,
        };

        Ok((solution, metadata))
    }

    /// Generate a solution using the model
    async fn generate_solution(
        query: &str,
        system_prompt: &str,
        _temperature: f32,
        _max_tokens: usize,
        tag: &str,
        client: &dyn ModelClient,
    ) -> Result<(String, usize)> {
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
        prompt.set_log_tag(tag);

        // Stream the response
        let mut stream = client.stream(&prompt);
        let mut response_text = String::new();
        let mut total_tokens = 0;

        while let Some(event) = stream.next().await {
            match event {
                Ok(ResponseEvent::OutputTextDelta { delta }) => {
                    response_text.push_str(&delta);
                }
                Ok(ResponseEvent::Completed { token_usage }) => {
                    if let Some(usage) = token_usage {
                        total_tokens = usage.total_tokens();
                    }
                }
                Err(e) => {
                    return Err(MarsError::CoreError(format!(
                        "Failed to generate solution for {}: {}",
                        tag, e
                    )));
                }
            }
        }

        Ok((response_text, total_tokens))
    }

    /// Extract code from response (looking for code blocks)
    fn extract_code(text: &str) -> String {
        let code_regex = Regex::new(r"(?s)```(?:[\w-]+)?\n(.*?)```")
            .unwrap_or_else(|_| Regex::new(r"(?s)```(.*?)```").unwrap());

        if let Some(caps) = code_regex.captures(text) {
            caps.get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_else(|| text.to_string())
        } else {
            text.to_string()
        }
    }

    /// Check if two solutions are meaningfully different (normalized comparison)
    fn solutions_differ(sol1: &str, sol2: &str) -> bool {
        let extracted1 = Self::extract_code(sol1);
        let extracted2 = Self::extract_code(sol2);

        // Normalize whitespace and compare
        let norm1: String = extracted1.split_whitespace().collect();
        let norm2: String = extracted2.split_whitespace().collect();

        norm1 != norm2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = RTOConfig::new();
        assert_eq!(config.initial_temperature, 0.1);
        assert_eq!(config.max_tokens_initial, 4096);
    }

    #[test]
    fn test_config_with_temperature() {
        let config = RTOConfig::new().with_initial_temperature(0.5);
        assert_eq!(config.initial_temperature, 0.5);
    }

    #[test]
    fn test_config_with_all_temperatures() {
        let config = RTOConfig::new().with_all_temperatures(0.7);
        assert_eq!(config.initial_temperature, 0.7);
        assert_eq!(config.description_temperature, 0.7);
        assert_eq!(config.second_temperature, 0.7);
        assert_eq!(config.synthesis_temperature, 0.7);
    }

    #[test]
    fn test_config_validation_invalid_temperature() {
        let config = RTOConfig::new().with_initial_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_tokens() {
        let config = RTOConfig::new();
        let mut config = config;
        config.max_tokens_initial = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = RTOConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_extract_code_with_backticks() {
        let text = "Here is the code:\n```rust\nfn main() {}\n```";
        let extracted = RTOAggregator::extract_code(text);
        assert!(extracted.contains("fn main()"));
    }

    #[test]
    fn test_extract_code_without_backticks() {
        let text = "Here is the solution";
        let extracted = RTOAggregator::extract_code(text);
        assert_eq!(extracted, "Here is the solution");
    }

    #[test]
    fn test_solutions_differ_same() {
        let sol1 = "```\nfn main() { println!(\"hello\"); }\n```";
        let sol2 = "```\nfn main() { println!(\"hello\"); }\n```";
        assert!(!RTOAggregator::solutions_differ(sol1, sol2));
    }

    #[test]
    fn test_solutions_differ_different() {
        let sol1 = "```\nfn main() { println!(\"hello\"); }\n```";
        let sol2 = "```\nfn main() { println!(\"goodbye\"); }\n```";
        assert!(RTOAggregator::solutions_differ(sol1, sol2));
    }

    #[test]
    fn test_solutions_differ_whitespace_normalized() {
        let sol1 = "fn main() { println!(\"hello\"); }";
        let sol2 = "fn main() {\n    println!(\"hello\");\n}";
        assert!(!RTOAggregator::solutions_differ(sol1, sol2));
    }

    #[test]
    fn test_default_config() {
        let default_config = RTOConfig::default();
        let new_config = RTOConfig::new();
        assert_eq!(default_config.initial_temperature, new_config.initial_temperature);
        assert_eq!(default_config.max_tokens_initial, new_config.max_tokens_initial);
    }
}
