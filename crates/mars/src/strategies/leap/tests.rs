#[cfg(test)]
mod tests {
    use crate::strategies::leap::{LEAPConfig, LEAPAggregator, LEAPMetadata, Example, Mistake};

    #[test]
    fn test_leap_config_defaults() {
        let config = LEAPConfig::new();
        assert_eq!(config.extraction_temperature, 0.3);
        assert_eq!(config.mistake_temperature, 0.7);
        assert_eq!(config.principle_temperature, 0.3);
        assert_eq!(config.final_temperature, 0.5);
        assert_eq!(config.max_tokens_extraction, 2048);
        assert_eq!(config.max_tokens_mistakes, 2048);
        assert_eq!(config.max_tokens_principles, 2048);
        assert_eq!(config.max_tokens_final, 2048);
        assert_eq!(config.max_principles, 8);
    }

    #[test]
    fn test_leap_config_builder_chain() {
        let config = LEAPConfig::new()
            .with_extraction_temperature(0.4)
            .with_mistake_temperature(0.9)
            .with_principle_temperature(0.2)
            .with_final_temperature(0.6)
            .with_max_principles(10);

        assert_eq!(config.extraction_temperature, 0.4);
        assert_eq!(config.mistake_temperature, 0.9);
        assert_eq!(config.principle_temperature, 0.2);
        assert_eq!(config.final_temperature, 0.6);
        assert_eq!(config.max_principles, 10);
    }

    #[test]
    fn test_leap_config_validation_valid() {
        let config = LEAPConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_leap_config_validation_extraction_temp_too_high() {
        let config = LEAPConfig::new().with_extraction_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_extraction_temp_negative() {
        let config = LEAPConfig::new().with_extraction_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_mistake_temp_too_high() {
        let config = LEAPConfig::new().with_mistake_temperature(2.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_principle_temp_negative() {
        let config = LEAPConfig::new().with_principle_temperature(-0.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_final_temp_out_of_range() {
        let config = LEAPConfig::new().with_final_temperature(3.0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_zero_extraction_tokens() {
        let mut config = LEAPConfig::new();
        config.max_tokens_extraction = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_zero_mistake_tokens() {
        let mut config = LEAPConfig::new();
        config.max_tokens_mistakes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_zero_principle_tokens() {
        let mut config = LEAPConfig::new();
        config.max_tokens_principles = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_zero_final_tokens() {
        let mut config = LEAPConfig::new();
        config.max_tokens_final = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_config_validation_zero_max_principles() {
        let config = LEAPConfig::new().with_max_principles(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_leap_extract_output_basic() {
        let text = "Reasoning:\n<output>Final Answer</output>";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Final Answer");
    }

    #[test]
    fn test_leap_extract_output_multiline_content() {
        let text = "<output>\nPrinciple 1\nPrinciple 2\nPrinciple 3\n</output>";
        let result = LEAPAggregator::extract_output(text);
        assert!(result.contains("Principle 1"));
        assert!(result.contains("Principle 2"));
        assert!(result.contains("Principle 3"));
    }

    #[test]
    fn test_leap_extract_output_with_surrounding_text() {
        let text = "Before\n<output>Content</output>\nAfter";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Content");
    }

    #[test]
    fn test_leap_extract_output_no_closing_tag() {
        let text = "Some reasoning\n<output>Answer here";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Answer here");
    }

    #[test]
    fn test_leap_extract_output_empty_tags() {
        let text = "<output></output>";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "");
    }

    #[test]
    fn test_leap_extract_output_whitespace_trimming() {
        let text = "<output>   \n   Trimmed Content   \n   </output>";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Trimmed Content");
    }

    #[test]
    fn test_leap_extract_output_no_tags() {
        let text = "Just plain text without tags";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "");
    }

    #[test]
    fn test_leap_extract_output_nested_tags() {
        let text = "<output>Outer <inner>content</inner> text</output>";
        let result = LEAPAggregator::extract_output(text);
        assert!(result.contains("Outer"));
        assert!(result.contains("content"));
    }

    #[test]
    fn test_leap_metadata_creation() {
        let metadata = LEAPMetadata {
            total_tokens: 10000,
            examples_extracted: 5,
            mistakes_generated: 5,
            principles_learned: 3,
            final_principles: vec![
                "Principle 1".to_string(),
                "Principle 2".to_string(),
                "Principle 3".to_string(),
            ],
        };

        assert_eq!(metadata.total_tokens, 10000);
        assert_eq!(metadata.examples_extracted, 5);
        assert_eq!(metadata.mistakes_generated, 5);
        assert_eq!(metadata.principles_learned, 3);
        assert_eq!(metadata.final_principles.len(), 3);
    }

    #[test]
    fn test_leap_metadata_display() {
        let metadata = LEAPMetadata {
            total_tokens: 5000,
            examples_extracted: 2,
            mistakes_generated: 2,
            principles_learned: 4,
            final_principles: vec![],
        };

        let display_str = format!("{}", metadata);
        assert!(display_str.contains("5000"));
        assert!(display_str.contains("2"));
        assert!(display_str.contains("4"));
    }

    #[test]
    fn test_leap_example_creation() {
        let example = Example {
            question: "What is photosynthesis?".to_string(),
            answer: "Process where plants convert sunlight to energy".to_string(),
        };

        assert_eq!(example.question, "What is photosynthesis?");
        assert_eq!(
            example.answer,
            "Process where plants convert sunlight to energy"
        );
    }

    #[test]
    fn test_leap_mistake_creation() {
        let mistake = Mistake {
            question: "What is 5+3?".to_string(),
            reasoning: "5+3 = 9 because 5+4 would be 9...".to_string(),
            generated_answer: "9".to_string(),
            correct_answer: "8".to_string(),
        };

        assert_eq!(mistake.question, "What is 5+3?");
        assert_eq!(mistake.generated_answer, "9");
        assert_eq!(mistake.correct_answer, "8");
    }

    #[test]
    fn test_leap_config_default_equals_new() {
        let default_config = LEAPConfig::default();
        let new_config = LEAPConfig::new();

        assert_eq!(default_config.extraction_temperature, new_config.extraction_temperature);
        assert_eq!(default_config.mistake_temperature, new_config.mistake_temperature);
        assert_eq!(
            default_config.principle_temperature,
            new_config.principle_temperature
        );
        assert_eq!(default_config.final_temperature, new_config.final_temperature);
        assert_eq!(
            default_config.max_tokens_extraction,
            new_config.max_tokens_extraction
        );
    }

    #[test]
    fn test_leap_config_boundary_temperature_zero() {
        let config = LEAPConfig::new().with_extraction_temperature(0.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_leap_config_boundary_temperature_two() {
        let config = LEAPConfig::new().with_extraction_temperature(2.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_leap_extract_output_only_opening_tag() {
        let text = "Text before\n<output>\nContent here";
        let result = LEAPAggregator::extract_output(text);
        assert_eq!(result, "Content here");
    }

    #[test]
    fn test_leap_extract_output_multiple_tags() {
        let text = "<output>First</output> other text <output>Second</output>";
        let result = LEAPAggregator::extract_output(text);
        // Regex captures the first match
        assert_eq!(result, "First");
    }

    #[test]
    fn test_leap_metadata_empty_principles() {
        let metadata = LEAPMetadata {
            total_tokens: 1000,
            examples_extracted: 0,
            mistakes_generated: 0,
            principles_learned: 0,
            final_principles: vec![],
        };

        assert_eq!(metadata.final_principles.len(), 0);
        assert_eq!(metadata.principles_learned, 0);
    }
}
