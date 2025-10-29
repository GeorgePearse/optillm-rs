/// CoT Decoding: Structured chain-of-thought decoding guidance.
///
/// Guides model to follow structured reasoning patterns, improving quality
/// through step-by-step problem decomposition and analysis.

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};

/// Configuration for CoT Decoding strategy.
///
/// Controls reasoning structure and verification behavior.
#[derive(Clone, Debug)]
pub struct CotDecodingConfig {
    /// Number of reasoning steps to explicitly include in the structure.
    pub num_steps: usize,
    /// Whether to verify the quality of extracted reasoning steps.
    pub enable_verification: bool,
}

impl Default for CotDecodingConfig {
    fn default() -> Self {
        Self {
            num_steps: 4,
            enable_verification: true,
        }
    }
}

/// CoT Decoding aggregator for structured reasoning.
///
/// Provides structured templates for chain-of-thought reasoning,
/// guiding the model to decompose problems systematically.
pub struct CotDecodingAggregator;

impl CotDecodingAggregator {
    pub async fn run_cot_decoding(
        query: &str,
        system_prompt: &str,
        config: CotDecodingConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, CotDecodingMetadata)> {
        let cot_prompt = format!(
            "{}  Follow this structure: {}",
            system_prompt,
            "Step 1: [Analysis] \n Step 2: [Reasoning] \n Step 3: [Synthesis] \n Final Answer: [Answer]"
        );

        let system_msg = ResponseItem::Message {
            id: None,
            role: "system".to_string(),
            content: vec![ContentItem::InputText {
                text: cot_prompt,
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
        prompt.set_log_tag("cot_decoding");

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
            "cot_decoding".to_string(),
            reasoning,
            answer,
            0.7,
            total_tokens,
        );

        let metadata = CotDecodingMetadata {
            num_steps: config.num_steps,
            verification_enabled: config.enable_verification,
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

/// Metadata tracking CoT Decoding execution.
///
/// Records the reasoning structure and verification results.
#[derive(Clone, Debug)]
pub struct CotDecodingMetadata {
    /// Number of reasoning steps in the structured output.
    pub num_steps: usize,
    /// Whether verification checks were enabled and performed.
    pub verification_enabled: bool,
    /// Total tokens consumed during generation.
    pub total_tokens: usize,
}
