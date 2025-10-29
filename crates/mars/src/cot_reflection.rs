//! Chain of Thought Reflection Strategy
//!
//! This module implements the CoT Reflection optimization strategy which uses
//! a specialized prompt structure to encourage the model to:
//! 1. Think through the problem step by step
//! 2. Reflect on the thinking to check for errors
//! 3. Provide a final answer

use regex::Regex;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::{types::Solution, MarsError, Result};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for CoT Reflection strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CotReflectionConfig {
    /// Temperature for model generation
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: usize,
}

impl CotReflectionConfig {
    /// Create a new CoT Reflection configuration with default values
    pub fn new() -> Self {
        Self {
            temperature: 0.6,
            max_tokens: 4096,
        }
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(MarsError::InvalidConfiguration(
                "Temperature must be between 0.0 and 2.0".to_string(),
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

impl Default for CotReflectionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata from CoT Reflection execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CotReflectionMetadata {
    /// Total tokens used
    pub total_tokens: usize,
    /// The thinking process extracted from the response
    pub thinking: String,
    /// Whether the full response was used as fallback
    pub is_fallback: bool,
}

/// CoT Reflection Aggregator
pub struct CotReflectionAggregator;

impl CotReflectionAggregator {
    /// Run the CoT Reflection strategy
    pub async fn run_cot_reflection(
        query: &str,
        system_prompt: &str,
        config: CotReflectionConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, CotReflectionMetadata)> {
        config.validate()?;

        // Build the enhanced system prompt with CoT Reflection instructions
        let enhanced_prompt = Self::build_system_prompt(system_prompt);

        // Create system message
        let system_msg = ResponseItem::Message {
            id: None,
            role: "system".to_string(),
            content: vec![ContentItem::InputText {
                text: enhanced_prompt,
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
        prompt.set_log_tag("cot-reflection");

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
                        "Failed to generate CoT Reflection response: {}",
                        e
                    )));
                }
            }
        }

        // Extract thinking and output from the response
        let (thinking, output, is_fallback) = Self::extract_sections(&response_text);

        // Create the solution
        let solution = Solution::new(
            "cot_reflection".to_string(),
            thinking.clone(),
            output,
            config.temperature,
            total_tokens,
        );

        // Create metadata
        let metadata = CotReflectionMetadata {
            total_tokens,
            thinking,
            is_fallback,
        };

        Ok((solution, metadata))
    }

    /// Build the system prompt with CoT Reflection instructions
    fn build_system_prompt(system_prompt: &str) -> String {
        format!(
            r#"{}

You are an AI assistant that uses a Chain of Thought (CoT) approach with reflection to answer queries. Follow these steps:

1. Think through the problem step by step within the <thinking> tags.
2. Reflect on your thinking to check for any errors or improvements within the <reflection> tags.
3. Make any necessary adjustments based on your reflection.
4. Provide your final, concise answer within the <output> tags.

Important: The <thinking> and <reflection> sections are for your internal reasoning process only.
Do not include any part of the final answer in these sections.
The actual response to the query must be entirely contained within the <output> tags.

Use the following format for your response:
<thinking>
[Your step-by-step reasoning goes here. This is your internal thought process, not the final answer.]
<reflection>
[Your reflection on your reasoning, checking for errors or improvements]
</reflection>
[Any adjustments to your thinking based on your reflection]
</thinking>
<output>
[Your final, concise answer to the query. This is the only part that will be shown to the user.]
</output>
"#,
            system_prompt
        )
    }

    /// Extract thinking and output sections from the response
    fn extract_sections(response: &str) -> (String, String, bool) {
        let thinking_regex =
            Regex::new(r"(?s)<thinking>(.*?)</thinking>").expect("Invalid thinking regex");
        let output_regex = Regex::new(r"(?s)<output>(.*?)(?:</output>|$)").expect("Invalid output regex");

        let thinking = if let Some(caps) = thinking_regex.captures(response) {
            caps.get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_else(|| "No thinking process provided.".to_string())
        } else {
            "No thinking process provided.".to_string()
        };

        let (output, is_fallback) = if let Some(caps) = output_regex.captures(response) {
            (
                caps.get(1)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_else(|| response.to_string()),
                false,
            )
        } else {
            (response.to_string(), true)
        };

        (thinking, output, is_fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = CotReflectionConfig::new();
        assert_eq!(config.temperature, 0.6);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_config_with_temperature() {
        let config = CotReflectionConfig::new().with_temperature(0.8);
        assert_eq!(config.temperature, 0.8);
    }

    #[test]
    fn test_config_with_max_tokens() {
        let config = CotReflectionConfig::new().with_max_tokens(2048);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_config_validation_temperature_too_low() {
        let config = CotReflectionConfig::new().with_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_temperature_too_high() {
        let config = CotReflectionConfig::new().with_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_max_tokens() {
        let config = CotReflectionConfig::new().with_max_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = CotReflectionConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_extract_sections_with_both_tags() {
        let response = r#"
<thinking>
Let me think about this step by step.
<reflection>
This seems correct.
</reflection>
</thinking>
<output>
The answer is 42.
</output>
"#;
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(!is_fallback);
        assert!(thinking.contains("step by step"));
        assert_eq!(output.trim(), "The answer is 42.");
    }

    #[test]
    fn test_extract_sections_without_thinking_tag() {
        let response = "Just the answer";
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(is_fallback);
        assert_eq!(thinking, "No thinking process provided.");
        assert_eq!(output, "Just the answer");
    }

    #[test]
    fn test_extract_sections_without_output_tag() {
        let response = r#"
<thinking>
Let me think...
</thinking>
No output tags here"#;
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(is_fallback);
        assert!(thinking.contains("think"));
        assert_eq!(output, response);
    }

    #[test]
    fn test_extract_sections_incomplete_output_tag() {
        let response = r#"
<thinking>
Let me think...
</thinking>
<output>
The answer is incomplete
"#;
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(!is_fallback);
        assert!(thinking.contains("think"));
        assert!(output.contains("incomplete"));
    }

    #[test]
    fn test_build_system_prompt_includes_original() {
        let original = "Be helpful";
        let prompt = CotReflectionAggregator::build_system_prompt(original);
        assert!(prompt.contains("Be helpful"));
        assert!(prompt.contains("Chain of Thought"));
        assert!(prompt.contains("<thinking>"));
        assert!(prompt.contains("<output>"));
    }

    #[test]
    fn test_default_config() {
        let default_config = CotReflectionConfig::default();
        let new_config = CotReflectionConfig::new();
        assert_eq!(default_config.temperature, new_config.temperature);
        assert_eq!(default_config.max_tokens, new_config.max_tokens);
    }
}
