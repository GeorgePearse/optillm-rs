#[cfg(test)]
mod tests {
    use crate::strategies::pvg::{PVGConfig, PVGAggregator, PVGMetadata};

    #[test]
    fn test_pvg_config_defaults() {
        let config = PVGConfig::new();
        assert_eq!(config.num_solutions, 3);
        assert_eq!(config.num_rounds, 2);
        assert_eq!(config.initial_temperature, 0.7);
        assert_eq!(config.verification_temperature, 0.2);
        assert_eq!(config.refinement_temperature, 0.5);
    }

    #[test]
    fn test_pvg_config_builder() {
        let config = PVGConfig::new()
            .with_num_solutions(5)
            .with_num_rounds(3)
            .with_initial_temperature(0.8);

        assert_eq!(config.num_solutions, 5);
        assert_eq!(config.num_rounds, 3);
        assert_eq!(config.initial_temperature, 0.8);
    }

    #[test]
    fn test_pvg_config_validation_valid() {
        let config = PVGConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_pvg_config_validation_zero_solutions() {
        let mut config = PVGConfig::new();
        config.num_solutions = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pvg_config_validation_zero_rounds() {
        let mut config = PVGConfig::new();
        config.num_rounds = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pvg_config_validation_invalid_initial_temperature() {
        let config = PVGConfig::new().with_initial_temperature(2.5);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pvg_config_validation_invalid_verification_temperature() {
        let mut config = PVGConfig::new();
        config.verification_temperature = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pvg_config_validation_zero_max_tokens() {
        let mut config = PVGConfig::new();
        config.max_tokens_generation = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pvg_extract_score_with_score_label() {
        let response = "Based on the analysis:\nScore: 8.5\nExplanation: Good solution";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 8.5);
    }

    #[test]
    fn test_pvg_extract_score_case_insensitive() {
        let response = "SCORE: 7\nThis is good";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 7.0);
    }

    #[test]
    fn test_pvg_extract_score_integer() {
        let response = "Rating: 9 out of 10";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 9.0);
    }

    #[test]
    fn test_pvg_extract_score_decimal() {
        let response = "Confidence score: 6.75";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 6.75);
    }

    #[test]
    fn test_pvg_extract_score_clamped_above_max() {
        let response = "Score: 15";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 10.0);
    }

    #[test]
    fn test_pvg_extract_score_clamped_below_min() {
        let response = "Score: -5";
        let score = PVGAggregator::extract_score(response);
        assert!(score >= 0.0);
    }

    #[test]
    fn test_pvg_extract_score_first_number() {
        let response = "This 5 is better than 3 in my opinion";
        let score = PVGAggregator::extract_score(response);
        // Regex finds numbers, first 5
        assert!(score >= 3.0);
    }

    #[test]
    fn test_pvg_extract_score_no_number() {
        let response = "This solution is quite poor";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_pvg_extract_score_zero() {
        let response = "Score: 0";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_pvg_extract_score_ten() {
        let response = "Score: 10";
        let score = PVGAggregator::extract_score(response);
        assert_eq!(score, 10.0);
    }

    #[test]
    fn test_pvg_metadata_creation() {
        let metadata = PVGMetadata {
            total_tokens: 8000,
            rounds_executed: 2,
            final_scores: vec![7.5, 8.0, 9.5],
            best_score: 9.5,
            helpful_solutions_count: 6,
            sneaky_solutions_count: 6,
        };

        assert_eq!(metadata.total_tokens, 8000);
        assert_eq!(metadata.rounds_executed, 2);
        assert_eq!(metadata.best_score, 9.5);
        assert_eq!(metadata.final_scores.len(), 3);
        assert_eq!(metadata.helpful_solutions_count, 6);
        assert_eq!(metadata.sneaky_solutions_count, 6);
    }

    #[test]
    fn test_pvg_default_config_eq_new() {
        let default = PVGConfig::default();
        let new = PVGConfig::new();
        assert_eq!(default.num_solutions, new.num_solutions);
        assert_eq!(default.num_rounds, new.num_rounds);
        assert_eq!(default.initial_temperature, new.initial_temperature);
        assert_eq!(default.verification_temperature, new.verification_temperature);
    }
}
