use super::*;

#[test]
fn test_config_creation() {
    let config = BestOfNConfig::new(5);
    assert_eq!(config.num_candidates, 5);
    assert!(!config.temperatures.is_empty());
}

#[test]
fn test_config_with_custom_temps() {
    let temps = vec![0.2, 0.5, 0.8];
    let config = BestOfNConfig::new(3).with_temperatures(temps.clone());
    assert_eq!(config.temperatures, temps);
}

#[test]
fn test_config_with_selection_method() {
    let config = BestOfNConfig::new(5)
        .with_selection_method(SelectionMethod::HighestConfidence);

    match config.selection_method {
        SelectionMethod::HighestConfidence => (),
        _ => panic!("Selection method not set correctly"),
    }
}

#[test]
fn test_select_by_score() {
    let mut sol1 = Solution::new(
        "agent1".to_string(),
        "reasoning1".to_string(),
        "answer1".to_string(),
        0.3,
        100,
    );
    sol1.verification_score = 0.5;

    let mut sol2 = Solution::new(
        "agent2".to_string(),
        "reasoning2".to_string(),
        "answer2".to_string(),
        0.6,
        100,
    );
    sol2.verification_score = 0.8;

    let mut sol3 = Solution::new(
        "agent3".to_string(),
        "reasoning3".to_string(),
        "answer3".to_string(),
        0.9,
        100,
    );
    sol3.verification_score = 0.6;

    let solutions = vec![sol1, sol2, sol3];
    let (best_idx, _) = BestOfNAggregator::select_by_score(&solutions);
    assert_eq!(best_idx, 1); // sol2 has highest score
}

#[test]
fn test_select_by_confidence() {
    let sol1 = Solution::new(
        "agent1".to_string(),
        "short".to_string(),
        "answer1".to_string(),
        0.3,
        100,
    );

    let sol2 = Solution::new(
        "agent2".to_string(),
        "this is a much longer reasoning that should be more thorough".to_string(),
        "answer2".to_string(),
        0.6,
        100,
    );

    let solutions = vec![sol1, sol2];
    let (best_idx, _) = BestOfNAggregator::select_by_confidence(&solutions);
    assert_eq!(best_idx, 1); // sol2 has more thorough reasoning
}

#[test]
fn test_select_by_thoroughness() {
    let sol1 = Solution::new(
        "agent1".to_string(),
        "short reasoning".to_string(),
        "answer1".to_string(),
        0.3,
        100,
    );

    let sol2 = Solution::new(
        "agent2".to_string(),
        "this is a much longer and more thorough reasoning that should be preferred"
            .repeat(10),
        "answer2".to_string(),
        0.6,
        100,
    );

    let solutions = vec![sol1, sol2];
    let (best_idx, _) = BestOfNAggregator::select_by_thoroughness(&solutions);
    assert_eq!(best_idx, 1); // sol2 is longer
}

#[test]
fn test_select_by_conciseness() {
    let sol1 = Solution::new(
        "agent1".to_string(),
        "reasoning".to_string(),
        "a".to_string(),
        0.3,
        100,
    );

    let sol2 = Solution::new(
        "agent2".to_string(),
        "reasoning".to_string(),
        "this is a much longer answer".to_string(),
        0.6,
        100,
    );

    let solutions = vec![sol1, sol2];
    let (best_idx, _) = BestOfNAggregator::select_by_conciseness(&solutions);
    assert_eq!(best_idx, 0); // sol1 has shorter answer
}

#[test]
fn test_select_by_multi_criteria() {
    let mut sol1 = Solution::new(
        "agent1".to_string(),
        "short".to_string(),
        "a".to_string(),
        0.3,
        100,
    );
    sol1.verification_score = 0.9;

    let mut sol2 = Solution::new(
        "agent2".to_string(),
        "medium reasoning content here".to_string(),
        "longer answer here".to_string(),
        0.6,
        100,
    );
    sol2.verification_score = 0.7;

    let solutions = vec![sol1, sol2];
    let (best_idx, _) = BestOfNAggregator::select_by_multi_criteria(&solutions);
    // Should select based on combination of criteria
    assert!(best_idx < 2);
}

#[test]
fn test_parse_response_with_separator() {
    let response = "This is reasoning. Answer: This is the answer.";
    let (reasoning, answer) = BestOfNAggregator::parse_response(response);
    assert!(reasoning.contains("This is reasoning"));
    assert!(answer.contains("This is the answer"));
}

#[test]
fn test_parse_response_without_separator() {
    let response = "This is the complete response without any separator.";
    let (reasoning, answer) = BestOfNAggregator::parse_response(response);
    assert!(answer.len() > 0);
}

#[test]
fn test_parse_response_with_final_answer() {
    let response =
        "Let me think about this step by step. Final Answer: The answer is 42.";
    let (reasoning, answer) = BestOfNAggregator::parse_response(response);
    assert!(reasoning.contains("step by step"));
    assert!(answer.contains("42"));
}

#[test]
fn test_selection_statistics() {
    let mut sol1 = Solution::new("a1".to_string(), "r1".to_string(), "ans1".to_string(), 0.3, 100);
    sol1.verification_score = 0.5;

    let mut sol2 = Solution::new("a2".to_string(), "r2".to_string(), "ans2".to_string(), 0.6, 100);
    sol2.verification_score = 0.8;

    let mut sol3 = Solution::new("a3".to_string(), "r3".to_string(), "ans3".to_string(), 0.9, 100);
    sol3.verification_score = 0.6;

    let metadata = BestOfNMetadata {
        num_candidates: 3,
        total_tokens: 300,
        selection_method: "BestScore".to_string(),
        selection_score: 0.8,
        all_candidates: vec![sol1, sol2, sol3],
    };

    let stats = BestOfNAggregator::get_selection_statistics(&metadata);
    assert_eq!(stats.num_candidates, 3);
    assert!(stats.best_candidate_score >= stats.avg_candidate_score);
}

#[test]
fn test_empty_solutions_error() {
    let solutions: Vec<Solution> = vec![];
    let result = BestOfNAggregator::select_best_solution(&solutions, &SelectionMethod::BestScore);
    assert!(result.is_err());
}
