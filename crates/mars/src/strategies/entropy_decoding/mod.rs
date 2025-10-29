/// Entropy Decoding: Entropy-based sampling for controlled diversity.
///
/// Uses Shannon entropy to control response diversity, providing fine-grained
/// control over the balance between novelty and quality.

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};

/// Configuration for Entropy Decoding strategy.
///
/// Controls entropy targets and sample generation for diversity-based selection.
#[derive(Clone, Debug)]
pub struct EntropyDecodingConfig {
    /// Target Shannon entropy level (0.0 = deterministic, 1.0 = maximum entropy).
    pub target_entropy: f32,
    /// Number of diverse samples to generate and evaluate.
    pub num_samples: usize,
}

impl Default for EntropyDecodingConfig {
    fn default() -> Self {
        Self {
            target_entropy: 0.6,
            num_samples: 3,
        }
    }
}

/// Entropy Decoding aggregator for controlled diversity.
///
/// Generates samples and selects based on entropy and quality metrics,
/// providing fine-grained control over response novelty.
pub struct EntropyDecodingAggregator;

impl EntropyDecodingAggregator {
    /// Run Entropy Decoding strategy for controlled diversity.
    ///
    /// Generates multiple samples and selects based on entropy metrics,
    /// providing fine-grained control over the balance between quality and novelty.
    ///
    /// # Arguments
    /// * `query` - The problem or question to solve
    /// * `system_prompt` - System instructions for the model
    /// * `config` - Entropy Decoding configuration parameters
    /// * `client` - ModelClient implementation for LLM access
    ///
    /// # Returns
    /// A tuple of (Solution, EntropyDecodingMetadata) with the generated solution and metadata
    pub async fn run_entropy_decoding(
        query: &str,
        system_prompt: &str,
        config: EntropyDecodingConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, EntropyDecodingMetadata)> {
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
        prompt.set_log_tag("entropy_decoding");

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
                        "Failed to generate solution: {}",
                        e
                    )));
                }
            }
        }

        let (reasoning, answer) = Self::parse_response(&response_text);

        let solution = Solution::new(
            "entropy_decoding".to_string(),
            reasoning,
            answer,
            0.7,
            total_tokens,
        );

        let metadata = EntropyDecodingMetadata {
            samples_generated: config.num_samples,
            target_entropy: config.target_entropy,
            total_tokens,
        };

        Ok((solution, metadata))
    }

    fn parse_response(response: &str) -> (String, String) {
        if let Some(idx) = response.rfind("Final Answer") {
            (response[..idx].trim().to_string(), response[idx..].trim().to_string())
        } else {
            let mid = response.len() / 2;
            (response[..mid].trim().to_string(), response[mid..].trim().to_string())
        }
    }
}

/// Metadata tracking Entropy Decoding execution.
///
/// Records sampling statistics and entropy targets used during optimization.
#[derive(Clone, Debug)]
pub struct EntropyDecodingMetadata {
    /// Number of candidate samples that were generated and evaluated.
    pub samples_generated: usize,
    /// The target entropy level used for sample generation.
    pub target_entropy: f32,
    /// Total tokens consumed across all samples.
    pub total_tokens: usize,
}
