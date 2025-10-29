//! ReRead (RE2) strategy
//!
//! ReRead is a simple but effective strategy that re-reads a question before answering.
//! This encourages the model to reconsider and improve its response quality.

use serde::{Deserialize, Serialize};
use std::fmt;
use futures::StreamExt;

use crate::{MarsError, Result};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for ReRead strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReReadConfig {
    /// Temperature for the model response
    pub temperature: f32,
    /// Maximum tokens for response generation
    pub max_tokens: usize,
}

impl Default for ReReadConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 4096,
        }
    }
}

impl ReReadConfig {
    /// Create a new ReRead config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    /// Set maximum tokens
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_tokens = max;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(MarsError::InvalidConfiguration(
                "temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if self.max_tokens == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Result of ReRead execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReReadResult {
    /// Final answer after re-reading
    pub answer: String,
    /// Metadata about the execution
    pub metadata: ReReadMetadata,
}

/// Metadata from ReRead execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReReadMetadata {
    /// Total tokens used
    pub total_tokens: usize,
}

impl fmt::Display for ReReadMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReReadMetadata {{ tokens: {} }}", self.total_tokens)
    }
}

/// ReRead strategy aggregator
pub struct ReReadAggregator;

impl ReReadAggregator {
    /// Run the ReRead strategy
    pub async fn run_reread(
        query: &str,
        system_prompt: &str,
        config: ReReadConfig,
        client: &dyn ModelClient,
    ) -> Result<ReReadResult> {
        config.validate()?;

        // Create a prompt that encourages re-reading the question
        let prompt_text = format!(
            "{}\n\nRead the question again: {}",
            query, query
        );

        let (answer, tokens) =
            Self::generate_response(&prompt_text, system_prompt, &config, client)
                .await?;

        Ok(ReReadResult {
            answer,
            metadata: ReReadMetadata {
                total_tokens: tokens,
            },
        })
    }

    /// Generate a response from the model
    async fn generate_response(
        prompt_text: &str,
        system_prompt: &str,
        config: &ReReadConfig,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reread_config_default() {
        let config = ReReadConfig::default();
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_reread_config_new() {
        let config = ReReadConfig::new();
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_reread_config_builder() {
        let config = ReReadConfig::new()
            .with_temperature(0.5)
            .with_max_tokens(2048);

        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_reread_config_validation_valid() {
        let config = ReReadConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_validation_invalid_temperature() {
        let config = ReReadConfig::new().with_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_config_validation_negative_temperature() {
        let config = ReReadConfig::new().with_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_config_validation_zero_tokens() {
        let config = ReReadConfig::new().with_max_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_metadata_creation() {
        let metadata = ReReadMetadata {
            total_tokens: 1500,
        };

        assert_eq!(metadata.total_tokens, 1500);
    }

    #[test]
    fn test_reread_metadata_display() {
        let metadata = ReReadMetadata {
            total_tokens: 2000,
        };

        let display_str = format!("{}", metadata);
        assert!(display_str.contains("2000"));
    }

    #[test]
    fn test_reread_config_default_equals_new() {
        let default = ReReadConfig::default();
        let new = ReReadConfig::new();

        assert_eq!(default.temperature, new.temperature);
        assert_eq!(default.max_tokens, new.max_tokens);
    }

    #[test]
    fn test_reread_config_boundary_temperature_zero() {
        let config = ReReadConfig::new().with_temperature(0.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_boundary_temperature_two() {
        let config = ReReadConfig::new().with_temperature(2.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_large_max_tokens() {
        let config = ReReadConfig::new().with_max_tokens(100000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_single_token() {
        let config = ReReadConfig::new().with_max_tokens(1);
        assert!(config.validate().is_ok());
    }
}
