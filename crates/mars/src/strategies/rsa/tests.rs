#[cfg(test)]

    use super::*;

    #[test]
    fn test_config_creation() {
        let config = RSAConfig::new(5, 3, 2);
        assert_eq!(config.population_size, 5);
        assert_eq!(config.selection_size, 3);
        assert_eq!(config.num_iterations, 2);
    }

    #[test]
    fn test_config_with_criterion() {
        let config = RSAConfig::new(5, 3, 2)
            .with_selection_criterion(SelectionCriterion::Diversity);

        match config.selection_criterion {
            SelectionCriterion::Diversity => (),
            _ => panic!("Criterion not set"),
        }
    }

    #[test]
    fn test_select_by_score() {
        let mut sol1 = Solution::new("a1".to_string(), "r1".to_string(), "ans1".to_string(), 0.5, 100);
        sol1.verification_score = 0.5;

        let mut sol2 = Solution::new("a2".to_string(), "r2".to_string(), "ans2".to_string(), 0.5, 100);
        sol2.verification_score = 0.9;

        let mut sol3 = Solution::new("a3".to_string(), "r3".to_string(), "ans3".to_string(), 0.5, 100);
        sol3.verification_score = 0.7;

        let population = vec![sol1, sol2, sol3];
        let selected = RSAAggregator::select_by_score(&population, 2);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].verification_score, 0.9);
    }

    #[test]
    fn test_select_by_diversity() {
        let sol1 = Solution::new("a1".to_string(), "short".to_string(), "ans".to_string(), 0.5, 100);
        let sol2 = Solution::new("a2".to_string(), "medium reasoning here".to_string(), "ans".to_string(), 0.5, 100);
        let sol3 = Solution::new("a3".to_string(), "very long reasoning that takes up many characters".repeat(5).to_string(), "ans".to_string(), 0.5, 100);

        let population = vec![sol1, sol2, sol3];
        let selected = RSAAggregator::select_by_diversity(&population, 2);
        assert!(selected.len() >= 1);
    }

    #[test]
    fn test_find_best_solution() {
        let mut sol1 = Solution::new("a1".to_string(), "r1".to_string(), "ans1".to_string(), 0.5, 100);
        sol1.verification_score = 0.5;

        let mut sol2 = Solution::new("a2".to_string(), "r2".to_string(), "ans2".to_string(), 0.5, 100);
        sol2.verification_score = 0.9;

        let population = vec![sol1, sol2];
        let best = RSAAggregator::find_best_solution(&population);
        assert!(best.is_some());
        assert_eq!(best.unwrap().verification_score, 0.9);
    }

    #[test]
    fn test_calculate_avg_score() {
        let mut sol1 = Solution::new("a1".to_string(), "r1".to_string(), "ans1".to_string(), 0.5, 100);
        sol1.verification_score = 0.6;

        let mut sol2 = Solution::new("a2".to_string(), "r2".to_string(), "ans2".to_string(), 0.5, 100);
        sol2.verification_score = 0.8;

        let population = vec![sol1, sol2];
        let avg = RSAAggregator::calculate_avg_score(&population);
        assert!((avg - 0.7).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_diversity() {
        let sol1 = Solution::new("a1".to_string(), "r1".to_string(), "answer1".to_string(), 0.5, 100);
        let sol2 = Solution::new("a2".to_string(), "r2".to_string(), "answer2".to_string(), 0.5, 100);
        let sol3 = Solution::new("a3".to_string(), "r3".to_string(), "answer1".to_string(), 0.5, 100);

        let population = vec![sol1, sol2, sol3];
        let diversity = RSAAggregator::calculate_diversity(&population);
        assert_eq!(diversity, 2.0 / 3.0);
    }

    #[test]
    fn test_rsa_simple() {
        let mut sol1 = Solution::new("a1".to_string(), "r1".to_string(), "ans".to_string(), 0.5, 100);
        sol1.verification_score = 0.6;

        let mut sol2 = Solution::new("a2".to_string(), "r2".to_string(), "ans".to_string(), 0.5, 100);
        sol2.verification_score = 0.8;

        let config = RSAConfig::new(2, 1, 1);
        let result = RSAAggregator::run_rsa(&vec![sol1, sol2], config);
        assert!(result.is_ok());

        let (best, metadata) = result.unwrap();
        assert_eq!(metadata.total_iterations, 1);
        assert!(best.verification_score >= 0.6);
    }

    #[test]
    fn test_config_selection_size_clamping() {
        let config = RSAConfig::new(5, 10, 2); // selection_size > population_size
        assert!(config.selection_size <= config.population_size);
    }

    #[test]
    fn test_rsa_statistics() {
        let mut sol1 = Solution::new("a1".to_string(), "r1".to_string(), "ans".to_string(), 0.5, 100);
        sol1.verification_score = 0.6;

        let mut sol2 = Solution::new("a2".to_string(), "r2".to_string(), "ans".to_string(), 0.5, 100);
        sol2.verification_score = 0.8;

        let config = RSAConfig::new(2, 1, 1);
        let (_, metadata) = RSAAggregator::run_rsa(&vec![sol1, sol2], config).unwrap();

        let stats = RSAAggregator::get_statistics(&metadata);
        assert_eq!(stats.total_iterations, 1);
        assert!(stats.final_best_score >= 0.6);
    }
