#[cfg(test)]
mod tests {
    use crate::strategies::plansearch::{PlanSearchConfig, PlanSearchAggregator, PlanSearchMetadata};

    #[test]
    fn test_plansearch_config_defaults() {
        let config = PlanSearchConfig::new();
        assert_eq!(config.observation_temperature, 0.7);
        assert_eq!(config.solution_temperature, 0.7);
        assert_eq!(config.implementation_temperature, 0.1);
        assert_eq!(config.num_initial_observations, 3);
        assert_eq!(config.num_derived_observations, 2);
        assert_eq!(config.max_tokens_observations, 2048);
        assert_eq!(config.max_tokens_solution, 4096);
        assert_eq!(config.max_tokens_implementation, 4096);
    }

    #[test]
    fn test_plansearch_config_builder_chain() {
        let config = PlanSearchConfig::new()
            .with_observation_temperature(0.6)
            .with_solution_temperature(0.8)
            .with_implementation_temperature(0.2)
            .with_num_initial_observations(5)
            .with_num_derived_observations(3);

        assert_eq!(config.observation_temperature, 0.6);
        assert_eq!(config.solution_temperature, 0.8);
        assert_eq!(config.implementation_temperature, 0.2);
        assert_eq!(config.num_initial_observations, 5);
        assert_eq!(config.num_derived_observations, 3);
    }

    #[test]
    fn test_plansearch_config_validation_valid() {
        let config = PlanSearchConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_plansearch_config_validation_observation_temp_too_high() {
        let config = PlanSearchConfig::new().with_observation_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_observation_temp_negative() {
        let config = PlanSearchConfig::new().with_observation_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_solution_temp_out_of_range() {
        let config = PlanSearchConfig::new().with_solution_temperature(2.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_implementation_temp_negative() {
        let config = PlanSearchConfig::new().with_implementation_temperature(-0.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_zero_initial_observations() {
        let config = PlanSearchConfig::new().with_num_initial_observations(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_zero_observation_tokens() {
        let mut config = PlanSearchConfig::new();
        config.max_tokens_observations = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_zero_solution_tokens() {
        let mut config = PlanSearchConfig::new();
        config.max_tokens_solution = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_config_validation_zero_implementation_tokens() {
        let mut config = PlanSearchConfig::new();
        config.max_tokens_implementation = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_plansearch_extract_code_python() {
        let text = "Solution in Python:\n```python\ndef solve(n):\n    return n * 2\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("def solve"));
        assert!(code.contains("return n * 2"));
    }

    #[test]
    fn test_plansearch_extract_code_rust() {
        let text = "Here's the Rust implementation:\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("fn main"));
        assert!(code.contains("println!"));
    }

    #[test]
    fn test_plansearch_extract_code_javascript() {
        let text = "```javascript\nfunction solve(x) { return x + 1; }\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("function solve"));
    }

    #[test]
    fn test_plansearch_extract_code_no_language_specified() {
        let text = "Code block:\n```\nx = 5\ny = x + 10\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("x = 5"));
        assert!(code.contains("y = x + 10"));
    }

    #[test]
    fn test_plansearch_extract_code_plain_text() {
        let text = "This is just plain text without code blocks";
        let code = PlanSearchAggregator::extract_code(text);
        assert_eq!(code, text);
    }

    #[test]
    fn test_plansearch_extract_code_multiple_blocks() {
        let text = "First:\n```\ncode1\n```\nSecond:\n```\ncode2\n```";
        let code = PlanSearchAggregator::extract_code(text);
        // Extracts the first block
        assert!(code.contains("code1"));
    }

    #[test]
    fn test_plansearch_extract_code_with_whitespace() {
        let text = "```\n  \n  def foo():\n      pass\n  \n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("def foo"));
    }

    #[test]
    fn test_plansearch_metadata_creation() {
        let metadata = PlanSearchMetadata {
            total_tokens: 15000,
            observations_count: 5,
            initial_observations: vec![
                "Observation 1".to_string(),
                "Observation 2".to_string(),
                "Observation 3".to_string(),
            ],
            derived_observations: vec![
                "Derived 1".to_string(),
                "Derived 2".to_string(),
            ],
        };

        assert_eq!(metadata.total_tokens, 15000);
        assert_eq!(metadata.observations_count, 5);
        assert_eq!(metadata.initial_observations.len(), 3);
        assert_eq!(metadata.derived_observations.len(), 2);
    }

    #[test]
    fn test_plansearch_metadata_display() {
        let metadata = PlanSearchMetadata {
            total_tokens: 10000,
            observations_count: 4,
            initial_observations: vec![],
            derived_observations: vec![],
        };

        let display_str = format!("{}", metadata);
        assert!(display_str.contains("10000"));
        assert!(display_str.contains("4"));
    }

    #[test]
    fn test_plansearch_config_default_equals_new() {
        let default = PlanSearchConfig::default();
        let new = PlanSearchConfig::new();

        assert_eq!(default.observation_temperature, new.observation_temperature);
        assert_eq!(default.solution_temperature, new.solution_temperature);
        assert_eq!(
            default.implementation_temperature,
            new.implementation_temperature
        );
        assert_eq!(default.num_initial_observations, new.num_initial_observations);
        assert_eq!(default.num_derived_observations, new.num_derived_observations);
    }

    #[test]
    fn test_plansearch_config_boundary_temperatures() {
        let config_min = PlanSearchConfig::new()
            .with_observation_temperature(0.0)
            .with_solution_temperature(0.0)
            .with_implementation_temperature(0.0);
        assert!(config_min.validate().is_ok());

        let config_max = PlanSearchConfig::new()
            .with_observation_temperature(2.0)
            .with_solution_temperature(2.0)
            .with_implementation_temperature(2.0);
        assert!(config_max.validate().is_ok());
    }

    #[test]
    fn test_plansearch_extract_code_nested_backticks() {
        let text = "Solution:\n```\ncode with `backticks` inside\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert!(code.contains("code with"));
    }

    #[test]
    fn test_plansearch_metadata_empty_observations() {
        let metadata = PlanSearchMetadata {
            total_tokens: 1000,
            observations_count: 0,
            initial_observations: vec![],
            derived_observations: vec![],
        };

        assert_eq!(metadata.observations_count, 0);
        assert_eq!(metadata.initial_observations.len(), 0);
    }

    #[test]
    fn test_plansearch_config_many_observations() {
        let config = PlanSearchConfig::new()
            .with_num_initial_observations(100)
            .with_num_derived_observations(50);

        assert_eq!(config.num_initial_observations, 100);
        assert_eq!(config.num_derived_observations, 50);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_plansearch_extract_code_empty_block() {
        let text = "Empty code:\n```\n\n```";
        let code = PlanSearchAggregator::extract_code(text);
        // Should extract empty string from block
        assert!(code.is_empty() || code.trim().is_empty());
    }

    #[test]
    fn test_plansearch_extract_code_single_line() {
        let text = "```\nprint('hello')\n```";
        let code = PlanSearchAggregator::extract_code(text);
        assert_eq!(code, "print('hello')");
    }
}
