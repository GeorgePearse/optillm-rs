#[cfg(test)]

    use super::*;

    #[test]
    fn test_config_creation() {
        let config = SelfConsistencyConfig::new(5);
        assert_eq!(config.num_paths, 5);
        assert!(!config.temperatures.is_empty());
    }

    #[test]
    fn test_config_with_extraction_strategy() {
        let config = SelfConsistencyConfig::new(5)
            .with_extraction_strategy(AnswerExtractionStrategy::AfterMarker);

        match config.extraction_strategy {
            AnswerExtractionStrategy::AfterMarker => (),
            _ => panic!("Strategy not set"),
        }
    }

    #[test]
    fn test_extract_answer_last_line() {
        let text = "Let me think.\nThis is reasoning.\nThe answer is 42.";
        let answer =
            SelfConsistencyAggregator::extract_answer(text, &AnswerExtractionStrategy::LastLine);
        assert_eq!(answer, "The answer is 42.");
    }

    #[test]
    fn test_extract_answer_after_marker() {
        let text = "Some reasoning here. Answer: 42";
        let answer = SelfConsistencyAggregator::extract_answer(text, &AnswerExtractionStrategy::AfterMarker);
        assert!(answer.contains("42"));
    }

    #[test]
    fn test_extract_answer_last_sentence() {
        let text = "First sentence. Second sentence. The answer.";
        let answer = SelfConsistencyAggregator::extract_answer(text, &AnswerExtractionStrategy::LastSentence);
        assert!(answer.contains("answer") || answer.contains("The"));
    }

    #[test]
    fn test_extract_answer_in_quotes() {
        let text = "Some reasoning \"the answer is 42\" more text";
        let answer =
            SelfConsistencyAggregator::extract_answer(text, &AnswerExtractionStrategy::InQuotes);
        assert_eq!(answer, "the answer is 42");
    }

    #[test]
    fn test_majority_vote() {
        let mut votes = HashMap::new();
        votes.insert("42".to_string(), 3);
        votes.insert("43".to_string(), 1);
        votes.insert("44".to_string(), 1);

        let winner = SelfConsistencyAggregator::majority_vote(&votes);
        assert_eq!(winner, "42");
    }

    #[test]
    fn test_consensus_score_high() {
        let votes = {
            let mut v = HashMap::new();
            v.insert("correct".to_string(), 5);
            v
        };
        let score = SelfConsistencyAggregator::calculate_consensus_score(&votes, 5);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_consensus_score_split() {
        let votes = {
            let mut v = HashMap::new();
            v.insert("answer1".to_string(), 3);
            v.insert("answer2".to_string(), 2);
            v
        };
        let score = SelfConsistencyAggregator::calculate_consensus_score(&votes, 5);
        assert_eq!(score, 0.6);
    }

    #[test]
    fn test_config_with_voting_strategy() {
        let config = SelfConsistencyConfig::new(5)
            .with_voting_strategy(VotingStrategy::QualityWeighted);

        match config.voting_strategy {
            VotingStrategy::QualityWeighted => (),
            _ => panic!("Voting strategy not set"),
        }
    }

    #[test]
    fn test_config_with_consensus_threshold() {
        let config = SelfConsistencyConfig::new(5).with_consensus_threshold(0.75);
        assert_eq!(config.consensus_threshold, 0.75);
    }

    #[test]
    fn test_config_threshold_clamping() {
        let config = SelfConsistencyConfig::new(5).with_consensus_threshold(1.5);
        assert!(config.consensus_threshold <= 1.0);

        let config = SelfConsistencyConfig::new(5).with_consensus_threshold(-0.5);
        assert!(config.consensus_threshold >= 0.0);
    }

    #[test]
    fn test_voting_statistics() {
        let mut paths = vec![];
        for i in 0..3 {
            paths.push(ReasoningPath {
                id: i,
                reasoning: format!("reasoning {}", i),
                extracted_answer: "42".to_string(),
                temperature: 0.7,
                reasoning_length: 100,
            });
        }
        paths.push(ReasoningPath {
            id: 3,
            reasoning: "reasoning 3".to_string(),
            extracted_answer: "43".to_string(),
            temperature: 0.7,
            reasoning_length: 100,
        });

        let mut voting_results = HashMap::new();
        voting_results.insert("42".to_string(), 3);
        voting_results.insert("43".to_string(), 1);

        let metadata = SelfConsistencyMetadata {
            num_paths: 4,
            total_tokens: 500,
            extraction_strategy: "LastLine".to_string(),
            voting_strategy: "MajorityVote".to_string(),
            consensus_score: 0.75,
            voting_results,
            all_paths: paths,
            consensus_answer: "42".to_string(),
        };

        let stats = SelfConsistencyAggregator::get_voting_statistics(&metadata);
        assert_eq!(stats.total_paths, 4);
        assert_eq!(stats.unique_answers, 2);
        assert_eq!(stats.consensus_answer_votes, 3);
    }

    #[test]
    fn test_default_config() {
        let config = SelfConsistencyConfig::default();
        assert_eq!(config.num_paths, 5);
        assert_eq!(config.consensus_threshold, 0.5);
    }
