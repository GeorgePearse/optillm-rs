//! Integration tests for all optimization strategies
//!
//! Tests each strategy with a mock ModelClient to ensure proper functionality

use optillm_core::{ModelClient, Prompt, ResponseEvent};
use std::pin::Pin;
use futures::stream::Stream;

/// Mock ModelClient for testing
struct MockModelClient {
    responses: Vec<String>,
    response_idx: std::sync::atomic::AtomicUsize,
}

impl MockModelClient {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            response_idx: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn simple_response() -> String {
        "The capital of France is Paris.".to_string()
    }

    fn reasoning_response() -> String {
        "Thinking: Let me analyze this problem...\nFirst observation: The algorithm has O(n log n) complexity.\nFinal Answer: The time complexity is O(n log n)".to_string()
    }

    fn complex_response() -> String {
        "Step 1: Assume the hypothesis.\nStep 2: Derive the contradiction.\nFinal Answer: Therefore the statement is true.".to_string()
    }
}

impl ModelClient for MockModelClient {
    fn stream(
        &self,
        _prompt: &Prompt,
    ) -> Pin<Box<dyn Stream<Item = optillm_core::Result<ResponseEvent>> + Send>> {
        let idx = self.response_idx.load(std::sync::atomic::Ordering::SeqCst);
        let response = self.responses
            .get(idx % self.responses.len())
            .cloned()
            .unwrap_or_else(Self::simple_response);

        self.response_idx.store((idx + 1) % self.responses.len(), std::sync::atomic::Ordering::SeqCst);

        Box::pin(async_stream::stream! {
            yield Ok(ResponseEvent::OutputTextDelta {
                delta: response,
            });
            yield Ok(ResponseEvent::Completed {
                token_usage: Some(optillm_core::TokenUsage {
                    input_tokens: 10,
                    output_tokens: 50,
                }),
            });
        })
    }
}

#[tokio::test]
async fn test_autothink_with_mock_client() {
    use optillm_mars::strategies::{AutoThinkAggregator, AutoThinkConfig};

    let mock = MockModelClient::new(vec![
        MockModelClient::simple_response(),
        MockModelClient::reasoning_response(),
    ]);

    let config = AutoThinkConfig::default();
    let result = AutoThinkAggregator::run_autothink(
        "What is the capital of France?",
        "You are a helpful assistant.",
        config,
        &mock,
    ).await;

    assert!(result.is_ok());
    let (_solution, metadata) = result.unwrap();
    assert!(metadata.complexity_score >= 0.0 && metadata.complexity_score <= 1.0);
}

#[tokio::test]
async fn test_deep_thinking_with_mock_client() {
    use optillm_mars::strategies::{DeepThinkingAggregator, DeepThinkingConfig};

    let mock = MockModelClient::new(vec![
        MockModelClient::reasoning_response(),
        MockModelClient::reasoning_response(),
    ]);

    let config = DeepThinkingConfig {
        min_tokens: 100,
        max_tokens: 500,
        num_iterations: 2,
    };

    let result = DeepThinkingAggregator::run_deep_thinking(
        "Analyze the algorithm complexity",
        "You are a helpful assistant.",
        config,
        &mock,
    ).await;

    assert!(result.is_ok());
    let (_solution, metadata) = result.unwrap();
    assert_eq!(metadata.iterations_performed, 2);
    assert!(metadata.total_tokens > 0);
}

#[tokio::test]
async fn test_entropy_decoding_with_mock_client() {
    use optillm_mars::strategies::{EntropyDecodingAggregator, EntropyDecodingConfig};

    let mock = MockModelClient::new(vec![
        MockModelClient::reasoning_response(),
        MockModelClient::reasoning_response(),
        MockModelClient::reasoning_response(),
    ]);

    let config = EntropyDecodingConfig {
        target_entropy: 0.5,
        num_samples: 3,
    };

    let result = EntropyDecodingAggregator::run_entropy_decoding(
        "Generate diverse solutions",
        "You are a helpful assistant.",
        config,
        &mock,
    ).await;

    assert!(result.is_ok());
    let (_solution, metadata) = result.unwrap();
    assert_eq!(metadata.samples_generated, 3);
    assert_eq!(metadata.target_entropy, 0.5);
}

#[tokio::test]
async fn test_cot_decoding_with_mock_client() {
    use optillm_mars::strategies::{CotDecodingAggregator, CotDecodingConfig};

    let mock = MockModelClient::new(vec![
        MockModelClient::complex_response(),
    ]);

    let config = CotDecodingConfig {
        num_steps: 4,
        enable_verification: true,
    };

    let result = CotDecodingAggregator::run_cot_decoding(
        "Prove this theorem",
        "Follow structured reasoning.",
        config,
        &mock,
    ).await;

    assert!(result.is_ok());
    let (_solution, metadata) = result.unwrap();
    assert_eq!(metadata.num_steps, 4);
    assert!(metadata.verification_enabled);
}

#[tokio::test]
async fn test_r_star_with_mock_client() {
    use optillm_mars::strategies::{RStarAggregator, RStarConfig};

    let mock = MockModelClient::new(vec![
        MockModelClient::reasoning_response(),
        MockModelClient::reasoning_response(),
    ]);

    let config = RStarConfig {
        num_simulations: 2,
        exploration_constant: 1.414,
        num_candidates: 2,
    };

    let result = RStarAggregator::run_r_star(
        "Find optimal solution",
        "You are a helpful assistant.",
        config,
        &mock,
    ).await;

    assert!(result.is_ok());
    let (_solution, metadata) = result.unwrap();
    assert_eq!(metadata.simulations_run, 2);
    assert!(metadata.total_tokens > 0);
}

#[tokio::test]
async fn test_all_strategies_handle_empty_response() {
    use optillm_mars::strategies::*;

    let mock = MockModelClient::new(vec!["".to_string()]);

    // All strategies should handle empty responses gracefully
    let at_result = AutoThinkAggregator::run_autothink(
        "test",
        "test",
        AutoThinkConfig::default(),
        &mock,
    ).await;
    assert!(at_result.is_ok());

    let dt_result = DeepThinkingAggregator::run_deep_thinking(
        "test",
        "test",
        DeepThinkingConfig::default(),
        &mock,
    ).await;
    assert!(dt_result.is_ok());
}

#[test]
fn test_autothink_complexity_classification() {
    use optillm_mars::strategies::{AutoThinkOptimizer, AutoThinkConfig, ComplexityLevel};

    let optimizer = AutoThinkOptimizer::new(AutoThinkConfig::default());

    // Test simple query
    let simple = optimizer.classify_complexity("What is 2+2?");
    assert_eq!(simple, ComplexityLevel::Simple);

    // Test medium complexity query
    let medium = optimizer.classify_complexity(
        "Prove that the set of all prime numbers is infinite using contradiction"
    );
    assert_eq!(medium, ComplexityLevel::Medium);

    // Test complex query with multiple reasoning steps
    let complex = optimizer.classify_complexity(
        "Design a recursive algorithm with memoization to solve the knapsack problem, analyze its time complexity, and compare it with dynamic programming approaches while considering edge cases"
    );
    assert_eq!(complex, ComplexityLevel::Complex);
}

#[test]
fn test_temperature_selection() {
    use optillm_mars::strategies::{AutoThinkOptimizer, AutoThinkConfig, ComplexityLevel};

    let config = AutoThinkConfig {
        simple_temperature: 0.3,
        medium_temperature: 0.6,
        complex_temperature: 1.0,
        ..Default::default()
    };

    let optimizer = AutoThinkOptimizer::new(config);

    assert_eq!(optimizer.get_temperature(&ComplexityLevel::Simple), 0.3);
    assert_eq!(optimizer.get_temperature(&ComplexityLevel::Medium), 0.6);
    assert_eq!(optimizer.get_temperature(&ComplexityLevel::Complex), 1.0);
}
