/// AutoThink: Query complexity classification with adaptive reasoning depth.
///
/// AutoThink analyzes input query characteristics and automatically determines
/// optimal reasoning depth (shallow, medium, deep) with adaptive temperature
/// and iteration settings.

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};

/// Configuration for AutoThink strategy
#[derive(Clone, Debug)]
pub struct AutoThinkConfig {
    /// Simple query token threshold
    pub simple_token_threshold: usize,
    /// Complex query token threshold
    pub complex_token_threshold: usize,
    /// Temperature for simple queries
    pub simple_temperature: f32,
    /// Temperature for medium complexity
    pub medium_temperature: f32,
    /// Temperature for complex queries
    pub complex_temperature: f32,
}

impl Default for AutoThinkConfig {
    fn default() -> Self {
        Self {
            simple_token_threshold: 50,
            complex_token_threshold: 150,
            simple_temperature: 0.3,
            medium_temperature: 0.6,
            complex_temperature: 1.0,
        }
    }
}

/// Complexity level classification
#[derive(Clone, Debug, PartialEq)]
pub enum ComplexityLevel {
    Simple,
    Medium,
    Complex,
}

/// AutoThink optimizer
pub struct AutoThinkOptimizer {
    config: AutoThinkConfig,
}

impl AutoThinkOptimizer {
    /// Create new AutoThink optimizer
    pub fn new(config: AutoThinkConfig) -> Self {
        Self { config }
    }

    /// Classify query complexity
    pub fn classify_complexity(&self, query: &str) -> ComplexityLevel {
        let score = self.calculate_complexity_score(query);

        match score {
            s if s < 0.33 => ComplexityLevel::Simple,
            s if s < 0.67 => ComplexityLevel::Medium,
            _ => ComplexityLevel::Complex,
        }
    }

    /// Calculate complexity score
    fn calculate_complexity_score(&self, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut score = 0.0;

        // Token count contribution
        let token_count = words.len();
        let length_score = ((token_count as f32).log2() / 10.0).min(0.2);
        score += length_score;

        // Difficulty keywords
        let difficulty_keywords = [
            "prove", "derive", "analyze", "why", "how", "complex", "difficult",
            "algorithm", "optimize", "recursive",
        ];

        let keyword_matches = difficulty_keywords
            .iter()
            .filter(|kw| query_lower.contains(**kw))
            .count();

        let keyword_score = ((keyword_matches as f32 / 10.0) * 0.5).min(0.5);
        score += keyword_score;

        // Punctuation complexity
        let special_chars = query.matches('?').count() + query.matches(':').count();
        let punct_score = ((special_chars as f32 / 10.0) * 0.3).min(0.3);
        score += punct_score;

        score.min(1.0)
    }

    /// Get temperature for complexity level
    pub fn get_temperature(&self, complexity: &ComplexityLevel) -> f32 {
        match complexity {
            ComplexityLevel::Simple => self.config.simple_temperature,
            ComplexityLevel::Medium => self.config.medium_temperature,
            ComplexityLevel::Complex => self.config.complex_temperature,
        }
    }
}

/// AutoThink aggregator
pub struct AutoThinkAggregator;

impl AutoThinkAggregator {
    /// Run AutoThink on a query
    pub async fn run_autothink(
        query: &str,
        system_prompt: &str,
        config: AutoThinkConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, AutoThinkMetadata)> {
        let optimizer = AutoThinkOptimizer::new(config);
        let complexity = optimizer.classify_complexity(query);
        let temperature = optimizer.get_temperature(&complexity);

        // Create prompt with system and user messages
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
        prompt.set_log_tag("autothink");

        // Generate response
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

        // Parse response
        let (reasoning, answer) = Self::parse_response(&response_text);

        let solution = Solution::new(
            "autothink".to_string(),
            reasoning,
            answer,
            temperature,
            total_tokens,
        );

        let metadata = AutoThinkMetadata {
            complexity_level: format!("{:?}", complexity),
            complexity_score: optimizer.calculate_complexity_score(query),
            selected_temperature: temperature,
            total_tokens,
        };

        Ok((solution, metadata))
    }

    /// Parse response into reasoning and answer
    fn parse_response(response: &str) -> (String, String) {
        if let Some(answer_idx) = response
            .rfind("Final Answer")
            .or_else(|| response.rfind("Answer:"))
        {
            let reasoning = response[..answer_idx].trim().to_string();
            let answer = response[answer_idx..].trim().to_string();
            (reasoning, answer)
        } else {
            let mid = response.len() / 2;
            (
                response[..mid].trim().to_string(),
                response[mid..].trim().to_string(),
            )
        }
    }
}

/// Metadata for AutoThink execution
#[derive(Clone, Debug)]
pub struct AutoThinkMetadata {
    /// Classified complexity level
    pub complexity_level: String,
    /// Numerical complexity score
    pub complexity_score: f32,
    /// Selected temperature for generation
    pub selected_temperature: f32,
    /// Total tokens used
    pub total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_classification() {
        let config = AutoThinkConfig::default();
        let optimizer = AutoThinkOptimizer::new(config);

        let simple = "What is 2+2?";
        assert_eq!(optimizer.classify_complexity(simple), ComplexityLevel::Simple);

        let complex = "Prove that the sum of an infinite geometric series converges";
        assert_eq!(
            optimizer.classify_complexity(complex),
            ComplexityLevel::Complex
        );
    }

    #[test]
    fn test_temperature_selection() {
        let config = AutoThinkConfig::default();
        let optimizer = AutoThinkOptimizer::new(config);

        let temp_simple = optimizer.get_temperature(&ComplexityLevel::Simple);
        let temp_complex = optimizer.get_temperature(&ComplexityLevel::Complex);

        assert!(temp_simple < temp_complex);
    }
}
