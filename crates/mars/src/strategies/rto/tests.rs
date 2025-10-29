#[cfg(test)]
mod tests {
    use crate::strategies::rto::{RTOConfig, RTOAggregator};

    #[test]
    fn test_rto_config_defaults() {
        let config = RTOConfig::new();
        assert_eq!(config.initial_temperature, 0.1);
        assert_eq!(config.description_temperature, 0.1);
        assert_eq!(config.second_temperature, 0.1);
        assert_eq!(config.synthesis_temperature, 0.1);
        assert_eq!(config.max_tokens_initial, 4096);
        assert_eq!(config.max_tokens_description, 1024);
        assert_eq!(config.max_tokens_second, 4096);
        assert_eq!(config.max_tokens_synthesis, 4096);
    }

    #[test]
    fn test_rto_config_builder() {
        let config = RTOConfig::new()
            .with_initial_temperature(0.5)
            .with_all_temperatures(0.7);
        assert_eq!(config.initial_temperature, 0.7);
        assert_eq!(config.description_temperature, 0.7);
        assert_eq!(config.second_temperature, 0.7);
        assert_eq!(config.synthesis_temperature, 0.7);
    }

    #[test]
    fn test_rto_temperature_validation_low() {
        let config = RTOConfig::new().with_initial_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rto_temperature_validation_high() {
        let config = RTOConfig::new().with_initial_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rto_temperature_validation_valid() {
        let config = RTOConfig::new().with_all_temperatures(1.5);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rto_tokens_validation_zero_initial() {
        let mut config = RTOConfig::new();
        config.max_tokens_initial = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rto_tokens_validation_zero_description() {
        let mut config = RTOConfig::new();
        config.max_tokens_description = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rto_tokens_validation_zero_second() {
        let mut config = RTOConfig::new();
        config.max_tokens_second = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rto_tokens_validation_zero_synthesis() {
        let mut config = RTOConfig::new();
        config.max_tokens_synthesis = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_rto_extract_code_python() {
        let text = "Here's the Python code:\n```python\ndef hello():\n    print('hello')\n```";
        let extracted = RTOAggregator::extract_code(text);
        assert!(extracted.contains("def hello()"));
        assert!(extracted.contains("print('hello')"));
    }

    #[test]
    fn test_rto_extract_code_rust() {
        let text = "Here's the Rust code:\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let extracted = RTOAggregator::extract_code(text);
        assert!(extracted.contains("fn main()"));
        assert!(extracted.contains("println"));
    }

    #[test]
    fn test_rto_extract_code_no_language() {
        let text = "Code:\n```\nlet x = 5;\n```";
        let extracted = RTOAggregator::extract_code(text);
        assert!(extracted.contains("let x = 5"));
    }

    #[test]
    fn test_rto_extract_code_no_backticks() {
        let text = "Just plain text solution";
        let extracted = RTOAggregator::extract_code(text);
        assert_eq!(extracted, "Just plain text solution");
    }

    #[test]
    fn test_rto_solutions_identical() {
        let sol1 = "```\nfn main() {}\n```";
        let sol2 = "```\nfn main() {}\n```";
        assert!(!RTOAggregator::solutions_differ(sol1, sol2));
    }

    #[test]
    fn test_rto_solutions_different() {
        let sol1 = "```\nfn main() { println!(\"A\"); }\n```";
        let sol2 = "```\nfn main() { println!(\"B\"); }\n```";
        assert!(RTOAggregator::solutions_differ(sol1, sol2));
    }

    #[test]
    fn test_rto_solutions_whitespace_ignored() {
        let sol1 = "fn main() { println!(\"test\"); }";
        let sol2 = "fn main() {\n    println!(\"test\");\n}";
        // After normalization, should be identical
        assert!(!RTOAggregator::solutions_differ(sol1, sol2));
    }

    #[test]
    fn test_rto_solutions_multiline_extraction() {
        let sol1 = "```\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n```";
        let sol2 = "```\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n```";
        assert!(!RTOAggregator::solutions_differ(sol1, sol2));
    }

    #[test]
    fn test_rto_metadata_creation() {
        use crate::strategies::rto::RTOMetadata;

        let metadata = RTOMetadata {
            total_tokens: 1000,
            initial_solution: "C1".to_string(),
            description: "Q2".to_string(),
            second_solution: "C2".to_string(),
            solutions_differed: true,
            synthesized_solution: Some("C3".to_string()),
        };

        assert_eq!(metadata.total_tokens, 1000);
        assert!(metadata.solutions_differed);
        assert!(metadata.synthesized_solution.is_some());
    }
}
