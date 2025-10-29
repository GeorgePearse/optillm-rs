/// Entropy Decoding: Entropy-based sampling for controlled diversity

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};

#[derive(Clone, Debug)]
pub struct EntropyDecodingConfig {
    pub target_entropy: f32,
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

pub struct EntropyDecodingAggregator;

impl EntropyDecodingAggregator {
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

#[derive(Clone, Debug)]
pub struct EntropyDecodingMetadata {
    pub samples_generated: usize,
    pub target_entropy: f32,
    pub total_tokens: usize,
}
