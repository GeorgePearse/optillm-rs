#[cfg(test)]
mod tests {
    use crate::strategies::diverse_sampling::{
        DiverseSamplingConfig, DiverseSamplingAggregator, DiverseSamplingMetadata, Sample,
    };

    #[test]
    fn test_diverse_sampling_config_defaults() {
        let config = DiverseSamplingConfig::new();
        assert_eq!(config.num_samples, 5);
        assert_eq!(config.min_temperature, 0.3);
        assert_eq!(config.max_temperature, 1.5);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_diverse_sampling_config_builder_chain() {
        let config = DiverseSamplingConfig::new()
            .with_num_samples(8)
            .with_min_temperature(0.2)
            .with_max_temperature(1.8)
            .with_max_tokens(2048);

        assert_eq!(config.num_samples, 8);
        assert_eq!(config.min_temperature, 0.2);
        assert_eq!(config.max_temperature, 1.8);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_diverse_sampling_config_validation_valid() {
        let config = DiverseSamplingConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_validation_zero_samples() {
        let config = DiverseSamplingConfig::new().with_num_samples(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_min_temperature_negative() {
        let config = DiverseSamplingConfig::new().with_min_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_max_temperature_too_high() {
        let config = DiverseSamplingConfig::new().with_max_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_temps_reversed() {
        let config = DiverseSamplingConfig::new()
            .with_min_temperature(1.5)
            .with_max_temperature(0.3);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_zero_tokens() {
        let config = DiverseSamplingConfig::new().with_max_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_diverse_sampling_config_validation_boundary_zero_temp() {
        let config = DiverseSamplingConfig::new().with_min_temperature(0.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_validation_boundary_max_temp() {
        let config = DiverseSamplingConfig::new().with_max_temperature(2.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_equal_temperatures() {
        let config = DiverseSamplingConfig::new()
            .with_min_temperature(1.0)
            .with_max_temperature(1.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_single_sample() {
        let config = DiverseSamplingConfig::new().with_num_samples(1);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_many_samples() {
        let config = DiverseSamplingConfig::new().with_num_samples(100);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_sample_creation() {
        let sample = Sample {
            answer: "Test answer".to_string(),
            temperature: 0.7,
            tokens: 250,
        };

        assert_eq!(sample.answer, "Test answer");
        assert_eq!(sample.temperature, 0.7);
        assert_eq!(sample.tokens, 250);
    }

    #[test]
    fn test_diverse_sampling_multiple_samples() {
        let samples = vec![
            Sample {
                answer: "Answer A".to_string(),
                temperature: 0.3,
                tokens: 100,
            },
            Sample {
                answer: "Answer B".to_string(),
                temperature: 0.9,
                tokens: 120,
            },
            Sample {
                answer: "Answer C".to_string(),
                temperature: 1.5,
                tokens: 150,
            },
        ];

        assert_eq!(samples.len(), 3);
    }

    #[test]
    fn test_diverse_sampling_metadata_creation() {
        let samples = vec![
            Sample {
                answer: "Answer 1".to_string(),
                temperature: 0.3,
                tokens: 100,
            },
            Sample {
                answer: "Answer 2".to_string(),
                temperature: 0.9,
                tokens: 120,
            },
        ];

        let metadata = DiverseSamplingMetadata {
            total_tokens: 220,
            samples,
            unique_answers: 2,
        };

        assert_eq!(metadata.total_tokens, 220);
        assert_eq!(metadata.unique_answers, 2);
    }

    #[test]
    fn test_diverse_sampling_metadata_empty_samples() {
        let metadata = DiverseSamplingMetadata {
            total_tokens: 0,
            samples: vec![],
            unique_answers: 0,
        };

        assert_eq!(metadata.samples.len(), 0);
        assert_eq!(metadata.unique_answers, 0);
    }

    #[test]
    fn test_diverse_sampling_metadata_display() {
        let samples = vec![Sample {
            answer: "Test".to_string(),
            temperature: 0.5,
            tokens: 100,
        }];

        let metadata = DiverseSamplingMetadata {
            total_tokens: 3000,
            samples,
            unique_answers: 1,
        };

        let display_str = format!("{}", metadata);
        assert!(display_str.contains("3000"));
        assert!(display_str.contains("1"));
    }

    #[test]
    fn test_diverse_sampling_config_default_equals_new() {
        let default = DiverseSamplingConfig::default();
        let new = DiverseSamplingConfig::new();

        assert_eq!(default, new);
    }

    #[test]
    fn test_diverse_sampling_config_not_equal() {
        let config1 = DiverseSamplingConfig::new();
        let config2 = DiverseSamplingConfig::new().with_num_samples(10);

        assert_ne!(config1, config2);
    }

    #[test]
    fn test_diverse_sampling_config_boundary_single_sample() {
        let config = DiverseSamplingConfig::new().with_num_samples(1);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_large_num_samples() {
        let config = DiverseSamplingConfig::new().with_num_samples(1000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_large_max_tokens() {
        let config = DiverseSamplingConfig::new().with_max_tokens(100000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_narrow_temperature_range() {
        let config = DiverseSamplingConfig::new()
            .with_min_temperature(0.4)
            .with_max_temperature(0.5);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_diverse_sampling_config_wide_temperature_range() {
        let config = DiverseSamplingConfig::new()
            .with_min_temperature(0.0)
            .with_max_temperature(2.0);
        assert!(config.validate().is_ok());
    }
}
