/// Individual agents that explore solution paths with different temperatures.

use crate::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};
use crate::core::prompts;
use crate::types::Solution;
use crate::Result;
use futures::StreamExt;
use uuid::Uuid;

/// An individual agent in the MARS system
#[derive(Clone, Debug)]
pub struct Agent {
    /// Unique identifier for this agent
    pub id: String,
    /// Temperature setting for exploration (0.0 = deterministic, higher = more diverse)
    pub temperature: f32,
}

impl Agent {
    /// Create a new agent with the given temperature
    pub fn new(temperature: f32) -> Self {
        Self {
            id: format!("agent-{}", Uuid::new_v4()),
            temperature,
        }
    }

    /// Generate an initial solution given a query with ModelClient
    ///
    /// This method calls the LLM with appropriate prompting to generate
    /// a reasoning chain and answer to the given query.
    pub async fn generate_solution_with_client(
        &self,
        query: &str,
        use_thinking_tags: bool,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // Build the system and user prompts
        let system_prompt = if use_thinking_tags {
            prompts::MARS_SYSTEM_PROMPT_WITH_THINKING.to_string()
        } else {
            prompts::MARS_SYSTEM_PROMPT.to_string()
        };

        let user_prompt = format!(
            "{}\n\n{}",
            prompts::MARS_REASONING_PROMPT,
            query
        );

        // Build prompt for ModelClient
        let mut prompt = Prompt::default();
        prompt.input = vec![ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: user_prompt,
            }],
        }];
        prompt.base_instructions_override = Some(system_prompt);
        prompt.set_log_tag(&format!("mars_agent_{}", self.id));

        // Stream the response from LLM
        let mut stream = client.stream(&prompt);
        let mut full_response = String::new();
        let mut token_count = 0;

        while let Some(event) = stream.next().await {
            match event? {
                ResponseEvent::OutputTextDelta { delta, .. } => {
                    full_response.push_str(&delta);
                }
                ResponseEvent::Completed { token_usage, .. } => {
                    if let Some(usage) = token_usage {
                        token_count = usage.total_tokens() as usize;
                    }
                    break;
                }
            }
        }

        let (reasoning, answer) = self.parse_response(&full_response).await?;

        let solution = Solution::new(
            self.id.clone(),
            reasoning,
            answer,
            self.temperature,
            token_count,
        );

        Ok(solution)
    }

    /// Verify another agent's solution with ModelClient
    ///
    /// This method evaluates if a solution is mathematically correct,
    /// complete, and rigorous.
    pub async fn verify_solution_with_client(
        &self,
        solution: &Solution,
        client: &dyn ModelClient,
    ) -> Result<f32> {
        let verification_prompt = format!(
            "{}\n\nSolution to verify:\n{}\n\nAnswer: {}",
            prompts::VERIFICATION_SYSTEM_PROMPT, solution.reasoning, solution.answer
        );

        // Build prompt for ModelClient
        let mut prompt = Prompt::default();
        prompt.input = vec![ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: verification_prompt,
            }],
        }];
        prompt.set_log_tag(&format!("mars_verifier_{}", self.id));

        // Stream the verification response
        let mut stream = client.stream(&prompt);
        let mut verification_response = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ResponseEvent::OutputTextDelta { delta, .. } => {
                    verification_response.push_str(&delta);
                }
                ResponseEvent::Completed { .. } => {
                    break;
                }
            }
        }

        // Parse verification score from response
        let score = Self::extract_verification_score(&verification_response)?;
        Ok(score)
    }

    /// Improve an existing solution based on feedback with ModelClient
    ///
    /// This method takes unverified solutions and attempts to improve them
    /// based on verification feedback.
    pub async fn improve_solution_with_client(
        &self,
        solution: &Solution,
        feedback: &str,
        use_thinking_tags: bool,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        let system_prompt = if use_thinking_tags {
            prompts::MARS_SYSTEM_PROMPT_WITH_THINKING.to_string()
        } else {
            prompts::MARS_SYSTEM_PROMPT.to_string()
        };

        let improvement_prompt = format!(
            "{}\n\nOriginal solution:\nReasoning: {}\nAnswer: {}\n\nFeedback: {}\n\nPlease improve the solution:",
            prompts::IMPROVEMENT_PROMPT,
            solution.reasoning,
            solution.answer,
            feedback
        );

        // Build prompt for ModelClient
        let mut prompt = Prompt::default();
        prompt.input = vec![ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: improvement_prompt,
            }],
        }];
        prompt.base_instructions_override = Some(system_prompt);
        prompt.set_log_tag(&format!("mars_improve_{}", self.id));

        // Stream the improved response
        let mut stream = client.stream(&prompt);
        let mut improved_response = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ResponseEvent::OutputTextDelta { delta, .. } => {
                    improved_response.push_str(&delta);
                }
                ResponseEvent::Completed { .. } => {
                    break;
                }
            }
        }

        let (new_reasoning, new_answer) = self.parse_response(&improved_response).await?;

        let mut improved = Solution::new(
            self.id.clone(),
            new_reasoning,
            new_answer,
            self.temperature,
            solution.token_count,
        );

        improved.phase = crate::types::GenerationPhase::Improved;

        Ok(improved)
    }

    /// Extract strategies from a solution with ModelClient
    ///
    /// This identifies key techniques and approaches that worked well
    /// so other agents can benefit from them.
    pub async fn extract_strategies_with_client(
        &self,
        solution: &Solution,
        client: &dyn ModelClient,
    ) -> Result<Vec<String>> {
        let extraction_prompt = format!(
            "{}\n\nSolution:\n{}",
            prompts::STRATEGY_EXTRACTION_PROMPT, solution.reasoning
        );

        // Build prompt for ModelClient
        let mut prompt = Prompt::default();
        prompt.input = vec![ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: extraction_prompt,
            }],
        }];
        prompt.set_log_tag(&format!("mars_strategy_{}", self.id));

        // Stream the strategy extraction response
        let mut stream = client.stream(&prompt);
        let mut response = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ResponseEvent::OutputTextDelta { delta, .. } => {
                    response.push_str(&delta);
                }
                ResponseEvent::Completed { .. } => {
                    break;
                }
            }
        }

        // Parse strategies from response (numbered list format)
        let strategies = Self::parse_strategies(&response);
        Ok(strategies)
    }

    /// Parse a response into reasoning and answer components
    async fn parse_response(&self, response: &str) -> Result<(String, String)> {
        // Extract reasoning from <think> tags if present
        let reasoning = if let Some(start) = response.find("<think>") {
            if let Some(end) = response.find("</think>") {
                response[start + 7..end].trim().to_string()
            } else {
                response.to_string()
            }
        } else {
            response.to_string()
        };

        // Extract answer (everything after </think> or entire response)
        let answer = if let Some(end) = response.find("</think>") {
            response[end + 8..].trim().to_string()
        } else if let Some(pos) = response.find("---") {
            response[pos + 3..].trim().to_string()
        } else {
            // If no clear separator, use last 20% of response as answer
            let len = response.len();
            let split_point = (len * 4) / 5;
            response[split_point..].trim().to_string()
        };

        Ok((reasoning, answer))
    }

    /// Extract verification score from response
    fn extract_verification_score(response: &str) -> Result<f32> {
        // Look for "SCORE: 0.X" pattern
        let score = if let Some(score_start) = response.find("SCORE:") {
            let rest = &response[score_start + 6..];
            let score_str = rest.split_whitespace().next().unwrap_or("0.5");
            score_str
                .parse::<f32>()
                .unwrap_or(0.5)
                .max(0.0)
                .min(1.0)
        } else {
            // Default to reasonable score
            0.5
        };
        Ok(score)
    }

    /// Parse strategies from response
    fn parse_strategies(response: &str) -> Vec<String> {
        let mut strategies = Vec::new();

        // Look for numbered list items (1. , 2. , etc.)
        for line in response.lines() {
            let trimmed = line.trim();
            // Match lines starting with number. pattern
            if let Some(content) = trimmed
                .strip_prefix(|c: char| c.is_numeric())
                .and_then(|s| s.strip_prefix("."))
            {
                strategies.push(content.trim().to_string());
            }
        }

        // If no numbered strategies found, return placeholder
        if strategies.is_empty() {
            strategies.push("Use systematic reasoning".to_string());
            strategies.push("Break problem into components".to_string());
        }

        strategies
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self::new(0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new(0.7);
        assert_eq!(agent.temperature, 0.7);
        assert!(!agent.id.is_empty());
    }

    #[tokio::test]
    async fn test_agent_default() {
        let agent = Agent::default();
        assert_eq!(agent.temperature, 0.5);
    }
}
