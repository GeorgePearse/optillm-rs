//! PlanSearch strategy
//!
//! PlanSearch solves problems by generating observations, deriving new insights,
//! and using these to create both natural language and code solutions.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use futures::StreamExt;

use crate::{MarsError, Result};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for PlanSearch strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanSearchConfig {
    /// Temperature for observation generation
    pub observation_temperature: f32,
    /// Temperature for solution generation
    pub solution_temperature: f32,
    /// Temperature for implementation
    pub implementation_temperature: f32,
    /// Number of initial observations to generate
    pub num_initial_observations: usize,
    /// Number of derived observations to generate
    pub num_derived_observations: usize,
    /// Maximum tokens for observation generation
    pub max_tokens_observations: usize,
    /// Maximum tokens for solution generation
    pub max_tokens_solution: usize,
    /// Maximum tokens for implementation
    pub max_tokens_implementation: usize,
}

impl Default for PlanSearchConfig {
    fn default() -> Self {
        Self {
            observation_temperature: 0.7,
            solution_temperature: 0.7,
            implementation_temperature: 0.1,
            num_initial_observations: 3,
            num_derived_observations: 2,
            max_tokens_observations: 2048,
            max_tokens_solution: 4096,
            max_tokens_implementation: 4096,
        }
    }
}

impl PlanSearchConfig {
    /// Create a new PlanSearch config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set observation generation temperature
    pub fn with_observation_temperature(mut self, temp: f32) -> Self {
        self.observation_temperature = temp;
        self
    }

    /// Set solution generation temperature
    pub fn with_solution_temperature(mut self, temp: f32) -> Self {
        self.solution_temperature = temp;
        self
    }

    /// Set implementation temperature
    pub fn with_implementation_temperature(mut self, temp: f32) -> Self {
        self.implementation_temperature = temp;
        self
    }

    /// Set number of initial observations
    pub fn with_num_initial_observations(mut self, num: usize) -> Self {
        self.num_initial_observations = num;
        self
    }

    /// Set number of derived observations
    pub fn with_num_derived_observations(mut self, num: usize) -> Self {
        self.num_derived_observations = num;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=2.0).contains(&self.observation_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "observation_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.solution_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "solution_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.implementation_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "implementation_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if self.num_initial_observations == 0 {
            return Err(MarsError::InvalidConfiguration(
                "num_initial_observations must be greater than 0".to_string(),
            ));
        }
        if self.max_tokens_observations == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens_observations must be greater than 0".to_string(),
            ));
        }
        if self.max_tokens_solution == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens_solution must be greater than 0".to_string(),
            ));
        }
        if self.max_tokens_implementation == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens_implementation must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Result of PlanSearch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSearchResult {
    /// Natural language solution
    pub natural_language_solution: String,
    /// Code implementation
    pub code_implementation: String,
    /// Metadata about the execution
    pub metadata: PlanSearchMetadata,
}

/// Metadata from PlanSearch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSearchMetadata {
    /// Total tokens used
    pub total_tokens: usize,
    /// Number of observations generated
    pub observations_count: usize,
    /// Initial observations
    pub initial_observations: Vec<String>,
    /// Derived observations
    pub derived_observations: Vec<String>,
}

impl fmt::Display for PlanSearchMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PlanSearchMetadata {{ tokens: {}, observations: {} }}",
            self.total_tokens, self.observations_count
        )
    }
}

/// PlanSearch strategy aggregator
pub struct PlanSearchAggregator;

impl PlanSearchAggregator {
    /// Run the PlanSearch strategy
    pub async fn run_plansearch(
        problem: &str,
        system_prompt: &str,
        config: PlanSearchConfig,
        client: &dyn ModelClient,
    ) -> Result<PlanSearchResult> {
        config.validate()?;

        let mut total_tokens = 0;

        // Step 1: Generate initial observations
        let (initial_observations, tokens) = Self::generate_observations(
            problem,
            system_prompt,
            config.num_initial_observations,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        // Step 2: Generate derived observations
        let (derived_observations, tokens) = Self::generate_derived_observations(
            problem,
            &initial_observations,
            system_prompt,
            config.num_derived_observations,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        let all_observations: Vec<String> =
            initial_observations.iter().chain(derived_observations.iter()).cloned().collect();

        // Step 3: Generate natural language solution
        let (natural_language_solution, tokens) = Self::generate_solution(
            problem,
            &all_observations,
            system_prompt,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        // Step 4: Implement the solution
        let (code_implementation, tokens) = Self::implement_solution(
            problem,
            &natural_language_solution,
            system_prompt,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        Ok(PlanSearchResult {
            natural_language_solution,
            code_implementation,
            metadata: PlanSearchMetadata {
                total_tokens,
                observations_count: all_observations.len(),
                initial_observations,
                derived_observations,
            },
        })
    }

    /// Generate a response from the model
    async fn generate_response(
        prompt_text: &str,
        system_prompt: &str,
        max_tokens: usize,
        client: &dyn ModelClient,
    ) -> Result<(String, usize)> {
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
                text: prompt_text.to_string(),
            }],
        };

        let mut prompt = Prompt::new();
        prompt.input = vec![system_msg, user_msg];

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
                        "Failed to generate response: {}",
                        e
                    )));
                }
            }
        }

        Ok((response_text, total_tokens))
    }

    /// Generate initial observations about the problem
    async fn generate_observations(
        problem: &str,
        system_prompt: &str,
        num_observations: usize,
        config: &PlanSearchConfig,
        client: &dyn ModelClient,
    ) -> Result<(Vec<String>, usize)> {
        let prompt_text = format!(
            "You are an expert problem solver. You will be given a problem specification. \
             You will return several useful, non-obvious, and correct observations about the problem, \
             like hints to solve the problem. You will NOT return any code. Be as creative as possible, \
             going beyond what you think is intuitively correct.\n\n\
             Here is the problem:\n{}\n\n\
             Please provide {} observations.",
            problem, num_observations
        );

        let (response, tokens) =
            Self::generate_response(&prompt_text, system_prompt, config.max_tokens_observations, client)
                .await?;

        let observations: Vec<String> = response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect();

        Ok((observations, tokens))
    }

    /// Generate derived observations from initial ones
    async fn generate_derived_observations(
        problem: &str,
        initial_observations: &[String],
        system_prompt: &str,
        num_new_observations: usize,
        config: &PlanSearchConfig,
        client: &dyn ModelClient,
    ) -> Result<(Vec<String>, usize)> {
        let observations_text = initial_observations
            .iter()
            .enumerate()
            .map(|(i, obs)| format!("{}. {}", i + 1, obs))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt_text = format!(
            "You are an expert problem solver. You will be given a problem specification and \
             several correct observations about the problem. You will brainstorm several new, \
             useful, and correct observations about the problem, derived from the given observations. \
             You will NOT return any code. Be as creative as possible.\n\n\
             Here is the problem:\n{}\n\n\
             Here are the existing observations:\n{}\n\n\
             Please provide {} new observations derived from the existing ones.",
            problem, observations_text, num_new_observations
        );

        let (response, tokens) =
            Self::generate_response(&prompt_text, system_prompt, config.max_tokens_observations, client)
                .await?;

        let observations: Vec<String> = response
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect();

        Ok((observations, tokens))
    }

    /// Generate natural language solution using observations
    async fn generate_solution(
        problem: &str,
        observations: &[String],
        system_prompt: &str,
        config: &PlanSearchConfig,
        client: &dyn ModelClient,
    ) -> Result<(String, usize)> {
        let observations_text = observations
            .iter()
            .enumerate()
            .map(|(i, obs)| format!("Observation {}: {}", i + 1, obs))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt_text = format!(
            "Here is the problem:\n{}\n\n\
             Here are intelligent observations to help solve the problem:\n{}\n\n\
             Use these observations above to brainstorm a natural language solution to the problem above. \
             Note that your intuition may lead you astray, so come up with simple, creative ideas that \
             go beyond what you would usually come up with. \
             Quote relevant parts of the observations EXACTLY before each step of the solution. \
             QUOTING IS CRUCIAL.",
            problem, observations_text
        );

        Self::generate_response(&prompt_text, system_prompt, config.max_tokens_solution, client).await
    }

    /// Implement the solution in code
    async fn implement_solution(
        problem: &str,
        natural_language_solution: &str,
        system_prompt: &str,
        config: &PlanSearchConfig,
        client: &dyn ModelClient,
    ) -> Result<(String, usize)> {
        let prompt_text = format!(
            "You are an expert programmer. You will be given a problem specification and a \
             natural language solution/tutorial that describes how to solve the problem. \
             You will generate a correct program that matches said specification and tutorial. \
             You will NOT return anything except for the program inside markdown codeblocks.\n\n\
             Problem:\n{}\n\n\
             Solution:\n{}\n\n\
             Please implement the solution in code.",
            problem, natural_language_solution
        );

        let (response, tokens) =
            Self::generate_response(&prompt_text, system_prompt, config.max_tokens_implementation, client)
                .await?;

        // Extract code from markdown blocks if present
        let code = Self::extract_code(&response);
        Ok((code, tokens))
    }

    /// Extract code from markdown code blocks
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plansearch_config_default() {
        let config = PlanSearchConfig::default();
        assert_eq!(config.observation_temperature, 0.7);
        assert_eq!(config.solution_temperature, 0.7);
        assert_eq!(config.implementation_temperature, 0.1);
        assert_eq!(config.num_initial_observations, 3);
        assert_eq!(config.num_derived_observations, 2);
    }

    #[test]
    fn test_plansearch_config_new() {
        let config = PlanSearchConfig::new();
        assert_eq!(config.observation_temperature, 0.7);
        assert_eq!(config.num_initial_observations, 3);
    }

    #[test]
    fn test_plansearch_config_builder() {
        let config = PlanSearchConfig::new()
            .with_observation_temperature(0.5)
            .with_solution_temperature(0.6)
            .with_num_initial_observations(4);

        assert_eq!(config.observation_temperature, 0.5);
        assert_eq!(config.solution_temperature, 0.6);
        assert_eq!(config.num_initial_observations, 4);
    }

    #[test]
    fn test_plansearch_config_validation_valid() {
        let config = PlanSearchConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_plansearch_config_validation_invalid_observation_temp() {
        let config = PlanSearchConfig::new().with_observation_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_invalid_solution_temp() {
        let config = PlanSearchConfig::new().with_solution_temperature(-0.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_invalid_implementation_temp() {
        let config = PlanSearchConfig::new().with_implementation_temperature(3.0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_zero_observations() {
        let config = PlanSearchConfig::new().with_num_initial_observations(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_zero_tokens() {
        let mut config = PlanSearchConfig::new();
        config.max_tokens_observations = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_extract_code_with_language() {
        let text = "Here's the solution:\n```python\ndef solve():\n    return 42\n```\nDone";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("def solve"));
        assert!(code.contains("return 42"));
    }

    #[test]
    fn test_plansearch_extract_code_no_language() {
        let text = "Solution:\n```\nx = 5\ny = 10\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("x = 5"));
    }

    #[test]
    fn test_plansearch_extract_code_no_blocks() {
        let text = "Just plain text without code";
        let code = PlanSearchAggregator::extract_code(text);
        assert_eq!(code, text);
    }

    #[test]
    fn test_plansearch_metadata_creation() {
        let metadata = PlanSearchMetadata {
            total_tokens: 12000,
            observations_count: 5,
            initial_observations: vec!["Obs 1".to_string(), "Obs 2".to_string()],
            derived_observations: vec!["Derived 1".to_string()],
        };

        assert_eq!(metadata.total_tokens, 12000);
        assert_eq!(metadata.observations_count, 5);
        assert_eq!(metadata.initial_observations.len(), 2);
        assert_eq!(metadata.derived_observations.len(), 1);
    }

    #[test]
    fn test_plansearch_config_default_equals_new() {
        let default = PlanSearchConfig::default();
        let new = PlanSearchConfig::new();

        assert_eq!(default.observation_temperature, new.observation_temperature);
        assert_eq!(default.solution_temperature, new.solution_temperature);
        assert_eq!(
            default.implementation_temperature,
            new.implementation_temperature
        );
    }

    #[test]
    fn test_plansearch_extract_code_multiline() {
        let text = "Here's solution:\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("fn main()"));
        assert!(code.contains("println!"));
    }

    #[test]
    fn test_plansearch_config_boundary_temperature() {
        let config_zero = PlanSearchConfig::new().with_observation_temperature(0.0);
        assert!(config_zero.validate().is_ok());

        let config_two = PlanSearchConfig::new().with_observation_temperature(2.0);
        assert!(config_two.validate().is_ok());
    }
}
