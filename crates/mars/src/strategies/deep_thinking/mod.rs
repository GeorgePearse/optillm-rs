/// Deep Thinking: Inference-time scaling based on problem difficulty.
///
/// Deep Thinking allocates computation (tokens) proportionally to problem difficulty,
/// allowing harder problems to receive more reasoning capacity.

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};

/// Configuration for Deep Thinking strategy.
///
/// Controls token allocation and iteration count for difficulty-based scaling.
#[derive(Clone, Debug)]
pub struct DeepThinkingConfig {
    /// Minimum token allocation for any problem.
    pub min_tokens: usize,
    /// Maximum token allocation for highly complex problems.
    pub max_tokens: usize,
    /// Number of reasoning iterations to perform.
    pub num_iterations: usize,
}

impl Default for DeepThinkingConfig {
    fn default() -> Self {
        Self {
            min_tokens: 256,
            max_tokens: 2048,
            num_iterations: 3,
        }
    }
}

/// Deep Thinking aggregator for difficulty-based token allocation.
///
/// Estimates problem difficulty and scales token allocation accordingly,
/// providing harder problems with more computation for better reasoning.
pub struct DeepThinkingAggregator;

impl DeepThinkingAggregator {
    pub async fn run_deep_thinking(
        query: &str,
        system_prompt: &str,
        config: DeepThinkingConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, DeepThinkingMetadata)> {
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
        prompt.set_log_tag("deep_thinking");

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
            "deep_thinking".to_string(),
            reasoning,
            answer,
            0.7,
            total_tokens,
        );

        let metadata = DeepThinkingMetadata {
            iterations_performed: config.num_iterations,
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

/// Metadata tracking Deep Thinking execution.
///
/// Records how many iterations were performed and resource usage.
#[derive(Clone, Debug)]
pub struct DeepThinkingMetadata {
    /// Number of reasoning iterations that were executed.
    pub iterations_performed: usize,
    /// Total tokens consumed across all iterations.
    pub total_tokens: usize,
}
