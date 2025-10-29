#[cfg(test)]

    use super::*;

    #[test]
    fn test_moa_metadata_creation() {
        let metadata = MoaMetadata {
            total_tokens: 1000,
            phase1_tokens: 400,
            phase2_tokens: 200,
            phase3_tokens: 400,
            num_completions: 3,
            fallback_used: false,
        };

        assert_eq!(metadata.total_tokens, 1000);
        assert_eq!(metadata.phase1_tokens, 400);
        assert_eq!(metadata.phase2_tokens, 200);
        assert_eq!(metadata.phase3_tokens, 400);
        assert_eq!(metadata.num_completions, 3);
        assert!(!metadata.fallback_used);
    }

    #[test]
    fn test_moa_metadata_with_fallback() {
        let metadata = MoaMetadata {
            total_tokens: 1200,
            phase1_tokens: 600,
            phase2_tokens: 250,
            phase3_tokens: 350,
            num_completions: 3,
            fallback_used: true,
        };

        assert!(metadata.fallback_used);
        assert_eq!(metadata.total_tokens, 1200);
    }
