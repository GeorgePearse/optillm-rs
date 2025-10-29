#[cfg(test)]

    use super::*;

    #[test]
    fn test_config_creation() {
        let config = CotReflectionConfig::new();
        assert_eq!(config.temperature, 0.6);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_config_with_temperature() {
        let config = CotReflectionConfig::new().with_temperature(0.8);
        assert_eq!(config.temperature, 0.8);
    }

    #[test]
    fn test_config_with_max_tokens() {
        let config = CotReflectionConfig::new().with_max_tokens(2048);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_config_validation_temperature_too_low() {
        let config = CotReflectionConfig::new().with_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_temperature_too_high() {
        let config = CotReflectionConfig::new().with_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_max_tokens() {
        let config = CotReflectionConfig::new().with_max_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = CotReflectionConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_extract_sections_with_both_tags() {
        let response = r#"
<thinking>
Let me think about this step by step.
<reflection>
This seems correct.
</reflection>
</thinking>
<output>
The answer is 42.
</output>
"#;
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(!is_fallback);
        assert!(thinking.contains("step by step"));
        assert_eq!(output.trim(), "The answer is 42.");
    }

    #[test]
    fn test_extract_sections_without_thinking_tag() {
        let response = "Just the answer";
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(is_fallback);
        assert_eq!(thinking, "No thinking process provided.");
        assert_eq!(output, "Just the answer");
    }

    #[test]
    fn test_extract_sections_without_output_tag() {
        let response = r#"
<thinking>
Let me think...
</thinking>
No output tags here"#;
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(is_fallback);
        assert!(thinking.contains("think"));
        assert_eq!(output, response);
    }

    #[test]
    fn test_extract_sections_incomplete_output_tag() {
        let response = r#"
<thinking>
Let me think...
</thinking>
<output>
The answer is incomplete
"#;
        let (thinking, output, is_fallback) = CotReflectionAggregator::extract_sections(response);
        assert!(!is_fallback);
        assert!(thinking.contains("think"));
        assert!(output.contains("incomplete"));
    }

    #[test]
    fn test_build_system_prompt_includes_original() {
        let original = "Be helpful";
        let prompt = CotReflectionAggregator::build_system_prompt(original);
        assert!(prompt.contains("Be helpful"));
        assert!(prompt.contains("Chain of Thought"));
        assert!(prompt.contains("<thinking>"));
        assert!(prompt.contains("<output>"));
    }

    #[test]
    fn test_default_config() {
        let default_config = CotReflectionConfig::default();
        let new_config = CotReflectionConfig::new();
        assert_eq!(default_config.temperature, new_config.temperature);
        assert_eq!(default_config.max_tokens, new_config.max_tokens);
    }
