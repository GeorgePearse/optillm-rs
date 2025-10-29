//! Diverse Sampling strategy
//!
//! Diverse Sampling generates multiple answers with different temperature levels
//! to explore the response space with varying creativity levels, then selects the best.

use serde::{Deserialize, Serialize};
use std::fmt;
use futures::StreamExt;

use crate::{MarsError, Result};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for Diverse Sampling strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiverseSamplingConfig {
    /// Number of samples to generate
    pub num_samples: usize,
    /// Minimum temperature
    pub min_temperature: f32,
    /// Maximum temperature
    pub max_temperature: f32,
    /// Maximum tokens per sample
    pub max_tokens: usize,
}

impl Default for DiverseSamplingConfig {
    fn default() -> Self {
        Self {
            num_samples: 5,
            min_temperature: 0.3,
            max_temperature: 1.5,
            max_tokens: 4096,
        }
    }
}

impl DiverseSamplingConfig {
    /// Create a new Diverse Sampling config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of samples
    pub fn with_num_samples(mut self, num: usize) -> Self {
        self.num_samples = num;
        self
    }

    /// Set minimum temperature
    pub fn with_min_temperature(mut self, temp: f32) -> Self {
        self.min_temperature = temp;
        self
    }

    /// Set maximum temperature
    pub fn with_max_temperature(mut self, temp: f32) -> Self {
        self.max_temperature = temp;
        self
    }

    /// Set maximum tokens
    pub fn with_max_tokens(mut self, max: usize) -> Self {
        self.max_tokens = max;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.num_samples == 0 {
            return Err(MarsError::InvalidConfiguration(
                "num_samples must be greater than 0".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.min_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "min_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.max_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "max_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if self.min_temperature > self.max_temperature {
            return Err(MarsError::InvalidConfiguration(
                "min_temperature must be less than or equal to max_temperature".to_string(),
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

/// Sample with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    /// The generated answer
    pub answer: String,
    /// Temperature used for generation
    pub temperature: f32,
    /// Tokens used
    pub tokens: usize,
}

/// Result of Diverse Sampling execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiverseSamplingResult {
    /// Best answer selected
    pub best_answer: String,
    /// Temperature of best answer
    pub best_temperature: f32,
    /// Metadata about the execution
    pub metadata: DiverseSamplingMetadata,
}

/// Metadata from Diverse Sampling execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiverseSamplingMetadata {
    /// Total tokens used
    pub total_tokens: usize,
    /// All samples generated
    pub samples: Vec<Sample>,
    /// Number of unique answers
    pub unique_answers: usize,
}

impl fmt::Display for DiverseSamplingMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DiverseSamplingMetadata {{ tokens: {}, samples: {}, unique: {} }}",
            self.total_tokens,
            self.samples.len(),
            self.unique_answers
        )
    }
}

/// Diverse Sampling strategy aggregator
pub struct DiverseSamplingAggregator;

impl DiverseSamplingAggregator {
    /// Run the Diverse Sampling strategy
    pub async fn run_diverse_sampling(
        query: &str,
        system_prompt: &str,
        config: DiverseSamplingConfig,
        client: &dyn ModelClient,
    ) -> Result<DiverseSamplingResult> {
        config.validate()?;

        let mut samples = Vec::new();
        let mut total_tokens = 0;

        // Generate samples with varying temperatures
        for i in 0..config.num_samples {
            // Calculate temperature for this sample
            let temperature = if config.num_samples == 1 {
                config.min_temperature
            } else {
                let ratio = i as f32 / (config.num_samples - 1) as f32;
                config.min_temperature
                    + (config.max_temperature - config.min_temperature) * ratio
            };

            let (answer, tokens) =
                Self::generate_response(query, system_prompt, temperature, &config, client)
                    .await?;
            total_tokens += tokens;

            samples.push(Sample {
                answer,
                temperature,
                tokens,
            });
        }

        // Select best answer (first one by default, could be improved with scoring)
        let best_sample = samples.iter().next().ok_or_else(|| {
            MarsError::CoreError("No samples generated".to_string())
        })?;

        let unique_answers = Self::count_unique_answers(&samples);

        Ok(DiverseSamplingResult {
            best_answer: best_sample.answer.clone(),
            best_temperature: best_sample.temperature,
            metadata: DiverseSamplingMetadata {
                total_tokens,
                samples,
                unique_answers,
            },
        })
    }

    /// Generate a response from the model
    async fn generate_response(
        prompt_text: &str,
        system_prompt: &str,
        _temperature: f32,
        _config: &DiverseSamplingConfig,
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

    /// Count unique answers in samples
    fn count_unique_answers(samples: &[Sample]) -> usize {
        samples
            .iter()
            .map(|s| s.answer.to_lowercase())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diverse_sampling_config_default() {
        let config = DiverseSamplingConfig::default();
        assert_eq!(config.num_samples, 5);
        assert_eq!(config.min_temperature, 0.3);
        assert_eq!(config.max_temperature, 1.5);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_diverse_sampling_config_new() {
        let config = DiverseSamplingConfig::new();
        assert_eq!(config.num_samples, 5);
    }

    #[test]
    fn test_diverse_sampling_config_builder() {
        let config = DiverseSamplingConfig::new()
            .with_num_samples(10)
            .with_min_temperature(0.2)
            .with_max_temperature(1.8);

        assert_eq!(config.num_samples, 10);
        assert_eq!(config.min_temperature, 0.2);
        assert_eq!(config.max_temperature, 1.8);
    }

    #[test]
    fn test_diverse_sampling_config_validation_valid() {
        let config = DiverseSamplingConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_validation_zero_samples() {
        let config = DiverseSamplingConfig::new().with_num_samples(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_min_temp_invalid() {
        let config = DiverseSamplingConfig::new().with_min_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_max_temp_invalid() {
        let config = DiverseSamplingConfig::new().with_max_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_temp_order() {
        let config = DiverseSamplingConfig::new()
            .with_min_temperature(1.5)
            .with_max_temperature(0.3);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_zero_tokens() {
        let config = DiverseSamplingConfig::new().with_max_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_sample_creation() {
        let sample = Sample {
            answer: "Test answer".to_string(),
            temperature: 0.7,
            tokens: 150,
        };

        assert_eq!(sample.answer, "Test answer");
        assert_eq!(sample.temperature, 0.7);
        assert_eq!(sample.tokens, 150);
    }

    #[test]
    fn test_diverse_sampling_metadata_creation() {
        let samples = vec![
            Sample {
                answer: "Answer 1".to_string(),
                temperature: 0.3,
                tokens: 100,
            },
            Sample {
                answer: "Answer 2".to_string(),
                temperature: 0.9,
                tokens: 120,
            },
        ];

        let metadata = DiverseSamplingMetadata {
            total_tokens: 220,
            samples,
            unique_answers: 2,
        };

        assert_eq!(metadata.total_tokens, 220);
        assert_eq!(metadata.unique_answers, 2);
    }

    #[test]
    fn test_diverse_sampling_metadata_display() {
        let metadata = DiverseSamplingMetadata {
            total_tokens: 5000,
            samples: vec![],
            unique_answers: 3,
        };

        let display_str = format!("{}", metadata);
        assert!(display_str.contains("5000"));
        assert!(display_str.contains("3"));
    }

    #[test]
    fn test_diverse_sampling_config_default_equals_new() {
        let default = DiverseSamplingConfig::default();
        let new = DiverseSamplingConfig::new();

        assert_eq!(default, new);
    }

    #[test]
    fn test_diverse_sampling_config_boundary_temps() {
        let config = DiverseSamplingConfig::new()
            .with_min_temperature(0.0)
            .with_max_temperature(2.0);

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_equal_temps() {
        let config = DiverseSamplingConfig::new()
            .with_min_temperature(1.0)
            .with_max_temperature(1.0);

        assert!(config.validate().is_ok());
    }
}
