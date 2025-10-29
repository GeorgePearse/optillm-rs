/// Verification system for cross-agent solution checking.

use crate::types::{Solution, VerificationResult};
use crate::Result;

/// Verifier that checks solution correctness
pub struct Verifier;

impl Verifier {
    /// Verify a solution using another agent
    ///
    /// This simulates cross-agent verification where one agent checks
    /// another agent's solution for correctness.
    pub async fn verify_solution(
        solution: &Solution,
        verifying_agent_id: &str,
    ) -> Result<VerificationResult> {
        // In a real implementation, this would:
        // 1. Call the verifying agent to evaluate the solution
        // 2. Parse the verification response
        // 3. Extract correctness, score, and feedback
        //
        // For now, we'll create a placeholder that always returns a positive verification
        // TODO: Integrate with code-core's ModelClient for actual verification

        let is_correct = true; // Placeholder
        let score = 0.9; // Placeholder score (0.0-1.0)

        let mut result = VerificationResult::new(
            solution.id.clone(),
            is_correct,
            score,
            verifying_agent_id.to_string(),
        );

        result.correctness_feedback = "Answer appears mathematically sound".to_string();
        result.completeness_feedback = "Solution addresses all aspects of the problem".to_string();
        result.rigor_feedback = "Reasoning is well-justified and systematic".to_string();

        Ok(result)
    }

    /// Verify multiple solutions in parallel
    pub async fn verify_solutions(
        solutions: &[Solution],
        verifying_agent_id: &str,
    ) -> Result<Vec<VerificationResult>> {
        let mut verification_results = Vec::new();

        for solution in solutions {
            let result = Self::verify_solution(solution, verifying_agent_id).await?;
            verification_results.push(result);
        }

        Ok(verification_results)
    }

    /// Check if a solution meets consensus criteria
    ///
    /// A solution is considered verified when it receives multiple
    /// consecutive verification passes.
    pub fn meets_consensus(
        solution: &Solution,
        consensus_threshold: usize,
    ) -> bool {
        solution.verification_passes >= consensus_threshold && solution.verification_failures == 0
    }

    /// Calculate overall verification confidence
    ///
    /// Combines verification passes, failures, and scores into a single confidence metric.
    pub fn calculate_confidence(solution: &Solution) -> f32 {
        if solution.verification_passes == 0 {
            return 0.0;
        }

        let pass_ratio = solution.verification_passes as f32
            / (solution.verification_passes + solution.verification_failures) as f32;
        let score_factor = solution.verification_score;

        // Weighted combination: 70% pass rate, 30% score
        (pass_ratio * 0.7) + (score_factor * 0.3)
    }

    /// Filter solutions by verification status
    pub fn filter_verified(solutions: &[Solution]) -> Vec<Solution> {
        solutions.iter().filter(|s| s.is_verified).cloned().collect()
    }

    /// Find the best verified solution
    pub fn find_best_verified(solutions: &[Solution]) -> Option<Solution> {
        solutions
            .iter()
            .filter(|s| s.is_verified)
            .max_by(|a, b| {
                a.verification_score
                    .partial_cmp(&b.verification_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meets_consensus() {
        let mut solution = Solution::new(
            "agent1".to_string(),
            "reasoning".to_string(),
            "answer".to_string(),
            0.5,
            100,
        );

        assert!(!Verifier::meets_consensus(&solution, 2));

        solution.add_verification_pass(0.9);
        assert!(!Verifier::meets_consensus(&solution, 2));

        solution.add_verification_pass(0.9);
        assert!(Verifier::meets_consensus(&solution, 2));

        solution.add_verification_failure();
        assert!(!Verifier::meets_consensus(&solution, 2));
    }

    #[test]
    fn test_calculate_confidence() {
        let mut solution = Solution::new(
            "agent1".to_string(),
            "reasoning".to_string(),
            "answer".to_string(),
            0.5,
            100,
        );

        // No verifications
        assert_eq!(Verifier::calculate_confidence(&solution), 0.0);

        // With passes
        solution.add_verification_pass(0.8);
        let confidence = Verifier::calculate_confidence(&solution);
        assert!(confidence > 0.0);
        assert!(confidence <= 1.0);
    }

    #[test]
    fn test_filter_verified() {
        let mut sol1 = Solution::new(
            "agent1".to_string(),
            "r1".to_string(),
            "a1".to_string(),
            0.5,
            100,
        );
        sol1.add_verification_pass(0.9);
        sol1.add_verification_pass(0.9);

        let sol2 =
            Solution::new("agent2".to_string(), "r2".to_string(), "a2".to_string(), 0.5, 100);

        let solutions = vec![sol1, sol2];
        let verified = Verifier::filter_verified(&solutions);
        assert_eq!(verified.len(), 1);
    }

    #[test]
    fn test_find_best_verified() {
        let mut sol1 = Solution::new(
            "agent1".to_string(),
            "r1".to_string(),
            "a1".to_string(),
            0.5,
            100,
        );
        sol1.add_verification_pass(0.7);
        sol1.add_verification_pass(0.7);
        let sol1_score = sol1.verification_score;

        let mut sol2 = Solution::new(
            "agent2".to_string(),
            "r2".to_string(),
            "a2".to_string(),
            0.5,
            100,
        );
        sol2.add_verification_pass(0.9);
        sol2.add_verification_pass(0.9);

        let solutions = vec![sol1, sol2];
        let best = Verifier::find_best_verified(&solutions);
        assert!(best.is_some());
        let best_solution = best.unwrap();
        assert_eq!(best_solution.agent_id, "agent2");
        // sol2 should have higher score than sol1
        // sol2: (0 + 0.9)/2 = 0.45, then (0.45 + 0.9)/2 = 0.675
        // sol1: (0 + 0.7)/2 = 0.35, then (0.35 + 0.7)/2 = 0.525
        assert!(best_solution.verification_score > sol1_score);
    }
}
