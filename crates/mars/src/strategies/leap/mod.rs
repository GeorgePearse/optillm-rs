//! LEAP (Learning from Errors for Adaptive Process) strategy
//!
//! LEAP extracts few-shot examples from queries, learns from intentional mistakes,
//! and applies derived principles to improve answer quality.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use futures::StreamExt;

use crate::{MarsError, Result};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

/// Configuration for LEAP strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LEAPConfig {
    /// Temperature for example extraction
    pub extraction_temperature: f32,
    /// Temperature for mistake generation
    pub mistake_temperature: f32,
    /// Temperature for principle generation
    pub principle_temperature: f32,
    /// Temperature for final answer generation
    pub final_temperature: f32,
    /// Maximum tokens for example extraction
    pub max_tokens_extraction: usize,
    /// Maximum tokens for mistake generation
    pub max_tokens_mistakes: usize,
    /// Maximum tokens for principle generation
    pub max_tokens_principles: usize,
    /// Maximum tokens for final answer
    pub max_tokens_final: usize,
    /// Maximum number of principles to keep
    pub max_principles: usize,
}

impl Default for LEAPConfig {
    fn default() -> Self {
        Self {
            extraction_temperature: 0.3,
            mistake_temperature: 0.7,
            principle_temperature: 0.3,
            final_temperature: 0.5,
            max_tokens_extraction: 2048,
            max_tokens_mistakes: 2048,
            max_tokens_principles: 2048,
            max_tokens_final: 2048,
            max_principles: 8,
        }
    }
}

impl LEAPConfig {
    /// Create a new LEAP config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set extraction temperature
    pub fn with_extraction_temperature(mut self, temp: f32) -> Self {
        self.extraction_temperature = temp;
        self
    }

    /// Set mistake generation temperature
    pub fn with_mistake_temperature(mut self, temp: f32) -> Self {
        self.mistake_temperature = temp;
        self
    }

    /// Set principle generation temperature
    pub fn with_principle_temperature(mut self, temp: f32) -> Self {
        self.principle_temperature = temp;
        self
    }

    /// Set final answer temperature
    pub fn with_final_temperature(mut self, temp: f32) -> Self {
        self.final_temperature = temp;
        self
    }

    /// Set maximum number of principles
    pub fn with_max_principles(mut self, max: usize) -> Self {
        self.max_principles = max;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=2.0).contains(&self.extraction_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "extraction_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.mistake_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "mistake_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.principle_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "principle_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.final_temperature) {
            return Err(MarsError::InvalidConfiguration(
                "final_temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if self.max_tokens_extraction == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens_extraction must be greater than 0".to_string(),
            ));
        }
        if self.max_tokens_mistakes == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens_mistakes must be greater than 0".to_string(),
            ));
        }
        if self.max_tokens_principles == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens_principles must be greater than 0".to_string(),
            ));
        }
        if self.max_tokens_final == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_tokens_final must be greater than 0".to_string(),
            ));
        }
        if self.max_principles == 0 {
            return Err(MarsError::InvalidConfiguration(
                "max_principles must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Result of LEAP execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LEAPResult {
    /// Final answer generated using learned principles
    pub answer: String,
    /// Metadata about the LEAP execution
    pub metadata: LEAPMetadata,
}

/// Metadata from LEAP execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LEAPMetadata {
    /// Total tokens used
    pub total_tokens: usize,
    /// Number of examples extracted
    pub examples_extracted: usize,
    /// Number of mistakes generated
    pub mistakes_generated: usize,
    /// Number of principles learned
    pub principles_learned: usize,
    /// Final principles applied
    pub final_principles: Vec<String>,
}

impl fmt::Display for LEAPMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LEAPMetadata {{ tokens: {}, examples: {}, mistakes: {}, principles: {} }}",
            self.total_tokens, self.examples_extracted, self.mistakes_generated, self.principles_learned
        )
    }
}

/// Example extracted from a query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// The question
    pub question: String,
    /// The correct answer
    pub answer: String,
}

/// Mistake generated from an example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mistake {
    /// Original question
    pub question: String,
    /// Generated reasoning with intentional error
    pub reasoning: String,
    /// Generated (incorrect) answer
    pub generated_answer: String,
    /// Correct answer
    pub correct_answer: String,
}

/// LEAP strategy aggregator
pub struct LEAPAggregator;

impl LEAPAggregator {
    /// Run the LEAP strategy
    pub async fn run_leap(
        query: &str,
        system_prompt: &str,
        config: LEAPConfig,
        client: &dyn ModelClient,
    ) -> Result<LEAPResult> {
        config.validate()?;

        let mut total_tokens = 0;

        // Step 1: Extract examples from query
        let (examples, tokens) = Self::extract_examples_from_query(
            query,
            system_prompt,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        let examples_extracted = examples.len();

        // If no examples found, just apply final generation
        if examples.is_empty() {
            let (answer, tokens) = Self::generate_final_answer(
                query,
                system_prompt,
                &[],
                &config,
                client,
            )
            .await?;
            total_tokens += tokens;

            return Ok(LEAPResult {
                answer,
                metadata: LEAPMetadata {
                    total_tokens,
                    examples_extracted: 0,
                    mistakes_generated: 0,
                    principles_learned: 0,
                    final_principles: vec![],
                },
            });
        }

        // Step 2: Generate mistakes for examples
        let (mistakes, tokens) = Self::generate_mistakes(
            &examples,
            system_prompt,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        let mistakes_generated = mistakes.len();

        // Step 3: Generate low-level principles from mistakes
        let (low_level_principles, tokens) = Self::generate_low_level_principles(
            &mistakes,
            system_prompt,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        // Step 4: Generate high-level principles
        let (high_level_principles, tokens) = Self::generate_high_level_principles(
            &low_level_principles,
            system_prompt,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        let principles_learned = high_level_principles.len();

        // Step 5: Apply principles to generate final answer
        let (answer, tokens) = Self::generate_final_answer(
            query,
            system_prompt,
            &high_level_principles,
            &config,
            client,
        )
        .await?;
        total_tokens += tokens;

        Ok(LEAPResult {
            answer,
            metadata: LEAPMetadata {
                total_tokens,
                examples_extracted,
                mistakes_generated,
                principles_learned,
                final_principles: high_level_principles,
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

    /// Extract few-shot examples from query
    async fn extract_examples_from_query(
        query: &str,
        system_prompt: &str,
        config: &LEAPConfig,
        client: &dyn ModelClient,
    ) -> Result<(Vec<Example>, usize)> {
        let prompt_text = format!(
            "Analyze the following query and determine if it contains few-shot examples. \
             If it does, extract the examples and their corresponding answers. \
             Format the examples as a JSON array of objects with 'question' and 'answer' fields. \
             If there are no examples, return an empty array. \
             Enclose your response within <output></output> tags.\n\nQuery: {}",
            query
        );

        let (response, tokens) =
            Self::generate_response(&prompt_text, system_prompt, config.max_tokens_extraction, client)
                .await?;

        let extracted = Self::extract_output(&response);
        let mut examples = Vec::new();

        if !extracted.is_empty() {
            if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&extracted) {
                for item in parsed {
                    if let (Some(question), Some(answer)) = (
                        item.get("question").and_then(|v| v.as_str()),
                        item.get("answer").and_then(|v| v.as_str()),
                    ) {
                        examples.push(Example {
                            question: question.to_string(),
                            answer: answer.to_string(),
                        });
                    }
                }
            }
        }

        Ok((examples, tokens))
    }

    /// Generate mistakes for given examples
    async fn generate_mistakes(
        examples: &[Example],
        system_prompt: &str,
        config: &LEAPConfig,
        client: &dyn ModelClient,
    ) -> Result<(Vec<Mistake>, usize)> {
        let mut mistakes = Vec::new();
        let mut total_tokens = 0;

        for example in examples {
            let prompt_text = format!(
                "Answer the following question step by step. To induce a mistake, \
                 deliberately introduce an error in your reasoning or calculation.\n\n\
                 Question: {}\n\n\
                 Provide your step-by-step reasoning, then enclose your final answer within \
                 <output></output> tags. Think step by step, but make sure to include a mistake.",
                example.question
            );

            let (response, tokens) =
                Self::generate_response(&prompt_text, system_prompt, config.max_tokens_mistakes, client)
                    .await?;
            total_tokens += tokens;

            let generated_answer = Self::extract_output(&response);

            if generated_answer != example.answer {
                mistakes.push(Mistake {
                    question: example.question.clone(),
                    reasoning: response.clone(),
                    generated_answer,
                    correct_answer: example.answer.clone(),
                });
            }
        }

        Ok((mistakes, total_tokens))
    }

    /// Generate low-level principles from mistakes
    async fn generate_low_level_principles(
        mistakes: &[Mistake],
        system_prompt: &str,
        config: &LEAPConfig,
        client: &dyn ModelClient,
    ) -> Result<(Vec<String>, usize)> {
        let mut principles = Vec::new();
        let mut total_tokens = 0;

        for mistake in mistakes {
            let prompt_text = format!(
                "Question: {}\n\
                 Generated Reasoning: {}\n\
                 Generated Answer: {}\n\
                 Correct Answer: {}\n\n\
                 Conduct a thorough analysis of the generated answer compared to the correct answer. \
                 Identify discrepancies, misunderstandings, or errors. Provide clear insights and \
                 principles that can improve future responses. Focus on general principles, not just \
                 this specific case.\n\n\
                 Enclose ONLY the principles within <output></output> tags.",
                mistake.question, mistake.reasoning, mistake.generated_answer, mistake.correct_answer
            );

            let (response, tokens) =
                Self::generate_response(&prompt_text, system_prompt, config.max_tokens_principles, client)
                    .await?;
            total_tokens += tokens;

            let principle = Self::extract_output(&response);

            if !principle.is_empty() {
                principles.push(principle);
            }
        }

        Ok((principles, total_tokens))
    }

    /// Generate high-level principles from low-level principles
    async fn generate_high_level_principles(
        low_level_principles: &[String],
        system_prompt: &str,
        config: &LEAPConfig,
        client: &dyn ModelClient,
    ) -> Result<(Vec<String>, usize)> {
        if low_level_principles.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let principles_text = low_level_principles.join("\n");

        let prompt_text = format!(
            "Low-level principles:\n{}\n\n\
             Create a list of unique and insightful principles to improve future responses \
             based on the analysis above. Focus on capturing the essence while eliminating redundancies. \
             Each point should be clear, concise, and directly derived from the analysis.\n\n\
             Create a numbered list of principles. Limit to at most {} principles.\n\
             Enclose your list within <output></output> tags.",
            principles_text, config.max_principles
        );

        let (response, tokens) =
            Self::generate_response(&prompt_text, system_prompt, config.max_tokens_principles, client)
                .await?;

        let principles_str = Self::extract_output(&response);

        let high_level_principles: Vec<String> = principles_str
            .split('\n')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        Ok((high_level_principles, tokens))
    }

    /// Generate final answer using learned principles
    async fn generate_final_answer(
        query: &str,
        system_prompt: &str,
        principles: &[String],
        config: &LEAPConfig,
        client: &dyn ModelClient,
    ) -> Result<(String, usize)> {
        let principles_text = if !principles.is_empty() {
            format!("Keep in mind these principles:\n{}\n\n", principles.join("\n"))
        } else {
            String::new()
        };

        let prompt_text = format!(
            "{}Please answer the following query:\n\n{}",
            principles_text, query
        );

        Self::generate_response(&prompt_text, system_prompt, config.max_tokens_final, client).await
    }

    /// Extract content between <output></output> tags
    fn extract_output(text: &str) -> String {
        let re = Regex::new(r"(?s)<output>(.*?)(?:</output>|$)").unwrap();
        if let Some(caps) = re.captures(text) {
            caps.get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leap_config_default() {
        let config = LEAPConfig::default();
        assert_eq!(config.extraction_temperature, 0.3);
        assert_eq!(config.mistake_temperature, 0.7);
        assert_eq!(config.principle_temperature, 0.3);
        assert_eq!(config.final_temperature, 0.5);
        assert_eq!(config.max_principles, 8);
    }

    #[test]
    fn test_leap_config_new() {
        let config = LEAPConfig::new();
        assert_eq!(config.extraction_temperature, 0.3);
        assert_eq!(config.max_principles, 8);
    }

    #[test]
    fn test_leap_config_builder() {
        let config = LEAPConfig::new()
            .with_extraction_temperature(0.5)
            .with_mistake_temperature(0.8)
            .with_max_principles(5);

        assert_eq!(config.extraction_temperature, 0.5);
        assert_eq!(config.mistake_temperature, 0.8);
        assert_eq!(config.max_principles, 5);
    }

    #[test]
    fn test_leap_config_validation_valid() {
        let config = LEAPConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_leap_config_validation_invalid_temperature() {
        let config = LEAPConfig::new().with_extraction_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_zero_tokens() {
        let mut config = LEAPConfig::new();
        config.max_tokens_extraction = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_zero_principles() {
        let mut config = LEAPConfig::new();
        config.max_principles = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_extract_output_with_tags() {
        let text = "Some text\n<output>Extracted content</output>\nMore text";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Extracted content");
    }

    #[test]
    fn test_leap_extract_output_multiline() {
        let text = "<output>\nLine 1\nLine 2\n</output>";
        let result = LEAPAggregator::extract_output(text);
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
    }

    #[test]
    fn test_leap_extract_output_no_closing_tag() {
        let text = "Some text\n<output>Content without closing tag";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Content without closing tag");
    }

    #[test]
    fn test_leap_extract_output_no_tags() {
        let text = "Just plain text";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "");
    }

    #[test]
    fn test_leap_metadata_creation() {
        let metadata = LEAPMetadata {
            total_tokens: 5000,
            examples_extracted: 3,
            mistakes_generated: 3,
            principles_learned: 5,
            final_principles: vec!["Principle 1".to_string(), "Principle 2".to_string()],
        };

        assert_eq!(metadata.total_tokens, 5000);
        assert_eq!(metadata.examples_extracted, 3);
        assert_eq!(metadata.mistakes_generated, 3);
        assert_eq!(metadata.principles_learned, 5);
        assert_eq!(metadata.final_principles.len(), 2);
    }

    #[test]
    fn test_leap_example_creation() {
        let example = Example {
            question: "What is 2+2?".to_string(),
            answer: "4".to_string(),
        };

        assert_eq!(example.question, "What is 2+2?");
        assert_eq!(example.answer, "4");
    }

    #[test]
    fn test_leap_mistake_creation() {
        let mistake = Mistake {
            question: "What is 2+2?".to_string(),
            reasoning: "2+2 = 5 because...".to_string(),
            generated_answer: "5".to_string(),
            correct_answer: "4".to_string(),
        };

        assert_eq!(mistake.question, "What is 2+2?");
        assert_eq!(mistake.generated_answer, "5");
        assert_eq!(mistake.correct_answer, "4");
    }

    #[test]
    fn test_leap_config_negative_temperature() {
        let config = LEAPConfig::new().with_extraction_temperature(-0.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_extract_output_empty() {
        let text = "<output></output>";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "");
    }

    #[test]
    fn test_leap_extract_output_with_whitespace() {
        let text = "<output>  \n  Content  \n  </output>";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Content");
    }
}
