#[cfg(test)]
mod tests {
    use crate::strategies::reread::{ReReadConfig, ReReadMetadata};

    #[test]
    fn test_reread_config_defaults() {
        let config = ReReadConfig::new();
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_reread_config_builder_chain() {
        let config = ReReadConfig::new()
            .with_temperature(0.8)
            .with_max_tokens(8192);

        assert_eq!(config.temperature, 0.8);
        assert_eq!(config.max_tokens, 8192);
    }

    #[test]
    fn test_reread_config_validation_valid() {
        let config = ReReadConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_validation_temperature_too_high() {
        let config = ReReadConfig::new().with_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_config_validation_temperature_negative() {
        let config = ReReadConfig::new().with_temperature(-0.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_config_validation_zero_max_tokens() {
        let config = ReReadConfig::new().with_max_tokens(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_config_validation_boundary_temp_zero() {
        let config = ReReadConfig::new().with_temperature(0.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_validation_boundary_temp_two() {
        let config = ReReadConfig::new().with_temperature(2.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_validation_high_tokens() {
        let config = ReReadConfig::new().with_max_tokens(1000000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_metadata_single_token() {
        let metadata = ReReadMetadata {
            total_tokens: 1,
        };

        assert_eq!(metadata.total_tokens, 1);
    }

    #[test]
    fn test_reread_metadata_large_tokens() {
        let metadata = ReReadMetadata {
            total_tokens: 1000000,
        };

        assert_eq!(metadata.total_tokens, 1000000);
    }

    #[test]
    fn test_reread_config_multiple_temperature_changes() {
        let config = ReReadConfig::new()
            .with_temperature(0.5)
            .with_temperature(0.9)
            .with_temperature(0.1);

        assert_eq!(config.temperature, 0.1);
    }

    #[test]
    fn test_reread_config_multiple_token_changes() {
        let config = ReReadConfig::new()
            .with_max_tokens(1000)
            .with_max_tokens(2000)
            .with_max_tokens(5000);

        assert_eq!(config.max_tokens, 5000);
    }

    #[test]
    fn test_reread_config_minimal_temperature() {
        let config = ReReadConfig::new().with_temperature(0.001);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_near_max_temperature() {
        let config = ReReadConfig::new().with_temperature(1.999);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_reread_config_just_above_max_temperature() {
        let config = ReReadConfig::new().with_temperature(2.001);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_config_just_below_zero_temperature() {
        let config = ReReadConfig::new().with_temperature(-0.001);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_reread_metadata_zero_tokens() {
        let metadata = ReReadMetadata {
            total_tokens: 0,
        };

        assert_eq!(metadata.total_tokens, 0);
    }

    #[test]
    fn test_reread_metadata_display_format() {
        let metadata = ReReadMetadata {
            total_tokens: 5500,
        };

        let display_str = format!("{}", metadata);
        assert!(display_str.contains("5500"));
        assert!(display_str.contains("ReReadMetadata"));
    }

    #[test]
    fn test_reread_config_default_equality() {
        let config1 = ReReadConfig::default();
        let config2 = ReReadConfig::new();

        assert_eq!(config1, config2);
    }

    #[test]
    fn test_reread_config_modified_inequality() {
        let config1 = ReReadConfig::new();
        let config2 = ReReadConfig::new().with_temperature(0.5);

        assert_ne!(config1, config2);
    }

    #[test]
    fn test_reread_config_common_temperatures() {
        let temps = vec![0.0, 0.3, 0.5, 0.7, 1.0, 1.4, 2.0];
        for temp in temps {
            let config = ReReadConfig::new().with_temperature(temp);
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_reread_config_common_token_limits() {
        let tokens = vec![1, 512, 1024, 2048, 4096, 8192, 16384];
        for max_tokens in tokens {
            let config = ReReadConfig::new().with_max_tokens(max_tokens);
            assert!(config.validate().is_ok());
        }
    }
}
