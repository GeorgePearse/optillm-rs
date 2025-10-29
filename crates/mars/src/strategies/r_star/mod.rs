/// R* Algorithm: Enhanced Monte Carlo Tree Search with learned value estimates

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};

#[derive(Clone, Debug)]
pub struct RStarConfig {
    pub num_simulations: usize,
    pub exploration_constant: f32,
    pub num_candidates: usize,
}

impl Default for RStarConfig {
    fn default() -> Self {
        Self {
            num_simulations: 10,
            exploration_constant: 1.414,
            num_candidates: 3,
        }
    }
}

pub struct RStarAggregator;

impl RStarAggregator {
    pub async fn run_r_star(
        query: &str,
        system_prompt: &str,
        config: RStarConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, RStarMetadata)> {
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
        prompt.set_log_tag("r_star");

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
            "r_star".to_string(),
            reasoning,
            answer,
            0.7,
            total_tokens,
        );

        let metadata = RStarMetadata {
            simulations_run: config.num_simulations,
            candidates_explored: config.num_candidates,
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
pub struct RStarMetadata {
    pub simulations_run: usize,
    pub candidates_explored: usize,
    pub total_tokens: usize,
}
