/// AutoThink: Query complexity classification with adaptive reasoning depth.
///
/// AutoThink analyzes input query characteristics and automatically determines
/// optimal reasoning depth (shallow, medium, deep) with adaptive temperature
/// and iteration settings.

use crate::{types::Solution, MarsError, Result};
use futures::StreamExt;
use optillm_core::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem};

/// Configuration for AutoThink strategy.
///
/// AutoThink adapts its behavior based on query complexity.
/// This configuration defines the thresholds and parameters for complexity classification.
#[derive(Clone, Debug)]
pub struct AutoThinkConfig {
    /// Token count threshold below which queries are considered simple.
    /// Queries with fewer tokens tend to be straightforward factual questions.
    pub simple_token_threshold: usize,
    /// Token count threshold above which queries are considered complex.
    /// Queries with more tokens typically require deeper reasoning.
    pub complex_token_threshold: usize,
    /// Temperature setting for simple queries (typically 0.0-0.5).
    /// Lower temperatures promote deterministic, focused responses.
    pub simple_temperature: f32,
    /// Temperature setting for medium complexity queries (typically 0.4-0.7).
    /// Medium temperatures balance consistency with some creative exploration.
    pub medium_temperature: f32,
    /// Temperature setting for complex queries (typically 0.7-1.0).
    /// Higher temperatures encourage diverse exploration for complex reasoning.
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

/// Complexity level classification for queries.
///
/// Queries are classified into three levels based on their characteristics,
/// such as token count, keyword presence, and structural complexity.
#[derive(Clone, Debug, PartialEq)]
pub enum ComplexityLevel {
    /// Simple queries requiring minimal reasoning (factual, straightforward).
    Simple,
    /// Medium complexity queries requiring moderate reasoning effort.
    Medium,
    /// Complex queries requiring extensive reasoning and analysis.
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
            s if s < 0.25 => ComplexityLevel::Simple,
            s if s < 0.40 => ComplexityLevel::Medium,
            _ => ComplexityLevel::Complex,
        }
    }

    /// Calculate complexity score using multi-factor analysis.
    ///
    /// Analyzes: length, vocabulary, keywords, domain indicators, and structure.
    fn calculate_complexity_score(&self, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut factor_weights = Vec::new();

        // 1. LENGTH ANALYSIS (20% weight)
        let length_score = self.analyze_length(&words);
        factor_weights.push(length_score * 0.20);

        // 2. VOCABULARY COMPLEXITY (25% weight)
        let vocab_score = self.analyze_vocabulary(&query_lower, &words);
        factor_weights.push(vocab_score * 0.25);

        // 3. REASONING KEYWORDS (25% weight)
        let reasoning_score = self.analyze_reasoning_keywords(&query_lower);
        factor_weights.push(reasoning_score * 0.25);

        // 4. DOMAIN INDICATORS (15% weight)
        let domain_score = self.analyze_domain_indicators(&query_lower);
        factor_weights.push(domain_score * 0.15);

        // 5. STRUCTURAL COMPLEXITY (15% weight)
        let structure_score = self.analyze_structure(query, &words);
        factor_weights.push(structure_score * 0.15);

        let score = factor_weights.iter().sum::<f32>();
        score.min(1.0)
    }

    /// Analyze query length with logarithmic scaling.
    fn analyze_length(&self, words: &[&str]) -> f32 {
        let word_count = words.len();
        match word_count {
            0..=10 => 0.0,      // Very short = simple
            11..=30 => 0.2,     // Short = somewhat simple
            31..=70 => 0.5,     // Medium = medium complexity
            71..=150 => 0.75,   // Long = more complex
            _ => 1.0,           // Very long = likely very complex
        }
    }

    /// Analyze vocabulary sophistication.
    fn analyze_vocabulary(&self, query_lower: &str, words: &[&str]) -> f32 {
        let mut vocab_score = 0.0;

        // Check for advanced vocabulary
        let advanced_words = [
            "theorem", "hypothesis", "methodology", "optimally", "recursively",
            "semantics", "pragmatics", "heuristic", "parameter", "architecture",
            "sophisticated", "intricate", "elaborate", "comprehensive", "rigorous",
        ];

        let advanced_count = advanced_words
            .iter()
            .filter(|word| query_lower.contains(**word))
            .count();
        vocab_score += ((advanced_count as f32 / 15.0) * 0.4).min(0.4);

        // Check average word length (longer words = more complex)
        let avg_word_len = words.iter().map(|w| w.len()).sum::<usize>() as f32 / words.len().max(1) as f32;
        let word_length_score = match avg_word_len {
            0.0..=4.0 => 0.0,       // Very short words
            4.1..=6.0 => 0.2,       // Short words
            6.1..=8.0 => 0.4,       // Medium words
            8.1..=10.0 => 0.6,      // Long words
            _ => 0.8,               // Very long words
        };
        vocab_score += word_length_score * 0.3;

        // Check for domain-specific jargon
        let jargon_words = [
            "matrix", "vector", "integral", "derivative", "probability",
            "algorithm", "data structure", "optimization", "neural", "tensor",
        ];
        let jargon_count = jargon_words
            .iter()
            .filter(|word| query_lower.contains(**word))
            .count();
        vocab_score += ((jargon_count as f32 / 10.0) * 0.3).min(0.3);

        vocab_score.min(1.0)
    }

    /// Analyze reasoning-related keywords.
    fn analyze_reasoning_keywords(&self, query_lower: &str) -> f32 {
        let reasoning_indicators = [
            ("prove", 1.0_f32),
            ("derive", 1.0_f32),
            ("demonstrate", 0.9_f32),
            ("explain", 0.7_f32),
            ("analyze", 0.8_f32),
            ("evaluate", 0.8_f32),
            ("compare", 0.7_f32),
            ("design", 0.9_f32),
            ("implement", 0.8_f32),
            ("optimize", 0.9_f32),
            ("complex", 0.7_f32),
            ("difficult", 0.8_f32),
            ("challenging", 0.7_f32),
            ("compute", 0.8_f32),
            ("solve", 0.6_f32),
            ("recursive", 0.95_f32),
            ("algorithm", 0.9_f32),
            ("edge case", 0.85_f32),
            ("corner case", 0.85_f32),
            ("formula", 0.7_f32),
            ("theory", 0.8_f32),
            ("abstract", 0.9_f32),
            ("contradiction", 0.9_f32),
            ("infinite", 0.9_f32),
        ];

        let mut max_score: f32 = 0.0;
        let mut match_count = 0;

        for (keyword, weight) in &reasoning_indicators {
            if query_lower.contains(keyword) {
                max_score = max_score.max(*weight);
                match_count += 1;
            }
        }

        // Combine max weight with match count
        let weight_score = max_score;
        let count_bonus = ((match_count as f32 / 24.0) * 0.3).min(0.3);
        (weight_score * 0.7 + count_bonus).min(1.0)
    }

    /// Analyze domain-specific indicators.
    fn analyze_domain_indicators(&self, query_lower: &str) -> f32 {
        let mut score: f32 = 0.0;

        // Mathematical domains
        let math_keywords = ["calculus", "algebra", "geometry", "topology", "group theory", "number theory"];
        let math_count = math_keywords.iter().filter(|kw| query_lower.contains(**kw)).count();
        if math_count > 0 {
            score += 0.8;
        }

        // Programming domains
        let prog_keywords = ["algorithm", "data structure", "dynamic programming", "sorting", "searching"];
        let prog_count = prog_keywords.iter().filter(|kw| query_lower.contains(**kw)).count();
        if prog_count > 1 {
            score += 0.7;
        }

        // Physics/Science domains
        let science_keywords = ["quantum", "relativity", "thermodynamics", "mechanics", "electromagnetism"];
        let sci_count = science_keywords.iter().filter(|kw| query_lower.contains(**kw)).count();
        if sci_count > 0 {
            score += 0.9;
        }

        // Logic/Philosophy domains
        let logic_keywords = ["logic", "inference", "proof", "axiom", "proposition", "theorem"];
        let logic_count = logic_keywords.iter().filter(|kw| query_lower.contains(**kw)).count();
        if logic_count > 0 {
            score += 0.85;
        }

        score.min(1.0)
    }

    /// Analyze structural complexity.
    fn analyze_structure(&self, query: &str, words: &[&str]) -> f32 {
        let mut score: f32 = 0.0;

        // Question mark count indicates inquiry depth
        let question_marks = query.matches('?').count();
        let multi_question_score = ((question_marks as f32 - 1.0) / 5.0).clamp(0.0, 0.3);
        score += multi_question_score;

        // Nested structures (parentheses, brackets)
        let parens = query.matches('(').count() + query.matches(')').count();
        let brackets = query.matches('[').count() + query.matches(']').count();
        let nesting_score = ((parens + brackets) as f32 / 20.0).clamp(0.0, 0.3);
        score += nesting_score;

        // Semicolons and colons indicate complex structure
        let colons = query.matches(':').count();
        let semis = query.matches(';').count();
        let separator_score = ((colons + semis) as f32 / 10.0).clamp(0.0, 0.2);
        score += separator_score;

        // Sentence complexity (longer sentences = more complex)
        let sentences: Vec<&str> = query.split('.').collect();
        let avg_sentence_len = if !sentences.is_empty() {
            words.len() as f32 / sentences.len() as f32
        } else {
            0.0
        };
        let sentence_score = match avg_sentence_len {
            0.0..=10.0 => 0.0,
            10.1..=20.0 => 0.15,
            20.1..=35.0 => 0.3,
            _ => 0.4,
        };
        score += sentence_score;

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

/// AutoThink aggregator for adaptive complexity-based reasoning.
///
/// Uses complexity classification to select optimal model parameters
/// and reasoning strategies for improved answer quality.
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

/// Metadata tracking AutoThink execution details.
///
/// Contains information about the complexity analysis and selected parameters
/// that were used during the optimization process.
#[derive(Clone, Debug)]
pub struct AutoThinkMetadata {
    /// Human-readable classification of query complexity level.
    /// Values: "Simple", "Medium", or "Complex".
    pub complexity_level: String,
    /// Numerical complexity score (0.0-1.0) calculated from query analysis.
    /// Higher scores indicate more complex queries.
    pub complexity_score: f32,
    /// The temperature value selected based on complexity analysis.
    /// Used to control the randomness/diversity of the model response.
    pub selected_temperature: f32,
    /// Total tokens consumed during generation.
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
