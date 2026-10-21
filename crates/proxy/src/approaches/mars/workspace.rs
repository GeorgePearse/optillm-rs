//! Port of upstream `optillm/mars/workspace.py`.

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use super::Config;

/// `AgentSolution`. `agent_id` is a string ("0", "agg_HHMMSS", "enhanced_0"),
/// while `MARSAgent.agent_id` is an int, exactly as upstream.
#[derive(Clone, Debug)]
pub struct AgentSolution {
    pub agent_id: String,
    pub solution: String,
    pub confidence: f64,
    /// `None` mirrors a Python `None` that leaked out of a usage getattr chain.
    pub reasoning_tokens: Option<i64>,
    pub is_verified: bool,
    pub verification_score: f64,
    pub temperature: f64,
    pub verification_results: Vec<VerificationRecord>,
}

/// The parts of a verification record that affect behaviour (issues and
/// reports only feed the unreachable improvement step upstream).
#[derive(Clone, Debug)]
pub struct VerificationRecord {
    pub assessment: String,
}

/// `VerificationResult`.
#[derive(Clone, Debug)]
pub struct VerificationResult {
    pub solution_id: String,
    pub assessment: String,
    pub confidence: f64,
}

pub struct Workspace {
    pub problem: String,
    pub config: Config,
    pub solutions: Vec<AgentSolution>,
    pub verification_results: Vec<VerificationResult>,
    pub iteration_count: i64,
    pub total_reasoning_tokens: Option<i64>,
}

impl Workspace {
    pub fn new(problem: &str, config: Config) -> Self {
        Self {
            problem: problem.to_string(),
            config,
            solutions: Vec::new(),
            verification_results: Vec::new(),
            iteration_count: 0,
            total_reasoning_tokens: Some(0),
        }
    }

    /// `add_solution` (adding a `None` token count raises like Python).
    pub fn add_solution(&mut self, s: AgentSolution) -> Result<()> {
        self.total_reasoning_tokens = match (self.total_reasoning_tokens, s.reasoning_tokens) {
            (Some(a), Some(b)) => Some(a + b),
            _ => {
                return Err(anyhow!(
                    "unsupported operand type(s) for +=: 'int' and 'NoneType'"
                ))
            }
        };
        self.solutions.push(s);
        Ok(())
    }

    /// `add_verification`.
    pub fn add_verification(&mut self, v: VerificationResult) {
        if v.solution_id.starts_with("agent_") {
            if let Some(agent_id_str) = v.solution_id.split('_').nth(1) {
                let threshold = self.config.int("verification_passes_required", 5);
                if let Some(solution) = self
                    .solutions
                    .iter_mut()
                    .find(|s| s.agent_id == agent_id_str)
                {
                    solution.verification_results.push(VerificationRecord {
                        assessment: v.assessment.clone(),
                    });
                    let verified = solution
                        .verification_results
                        .iter()
                        .filter(|r| r.assessment == "CORRECT")
                        .count();
                    let total = solution.verification_results.len();
                    solution.verification_score = if total > 0 {
                        verified as f64 / total as f64
                    } else {
                        0.0
                    };
                    let consecutive = solution
                        .verification_results
                        .iter()
                        .rev()
                        .take_while(|r| r.assessment == "CORRECT")
                        .count() as i64;
                    solution.is_verified = consecutive >= threshold;
                }
            }
        }
        self.verification_results.push(v);
    }

    pub fn verified_count(&self) -> usize {
        self.solutions.iter().filter(|s| s.is_verified).count()
    }

    /// `get_best_solution`.
    pub fn get_best_solution(&self) -> Option<&AgentSolution> {
        if self.solutions.is_empty() {
            return None;
        }
        let verified: Vec<&AgentSolution> =
            self.solutions.iter().filter(|s| s.is_verified).collect();
        if !verified.is_empty() {
            let idx = crate::py::strings::argmax_by(&verified, |s| s.confidence)?;
            return Some(verified[idx]);
        }
        let idx = crate::py::strings::argmax_by(&self.solutions, |s| s.verification_score)?;
        Some(&self.solutions[idx])
    }

    pub fn has_consensus(&self) -> bool {
        self.verified_count() as i64 >= self.config.int("consensus_threshold", 2)
    }

    pub fn should_continue_iteration(&self) -> bool {
        self.iteration_count < self.config.int("max_iterations", 5)
            && (self.verified_count() as i64) < self.config.int("min_verified_solutions", 1)
    }

    /// `_get_verification_summary` as an insertion-ordered dict.
    pub fn verification_summary(&self) -> Value {
        let total = self.verification_results.len();
        if total == 0 {
            return json!({"total": 0, "correct": 0, "incorrect": 0, "incomplete": 0});
        }
        let count = |a: &str| {
            self.verification_results
                .iter()
                .filter(|v| v.assessment == a)
                .count()
        };
        let mut m = Map::new();
        m.insert("total".into(), json!(total));
        m.insert("correct".into(), json!(count("CORRECT")));
        m.insert("incorrect".into(), json!(count("INCORRECT")));
        m.insert("incomplete".into(), json!(count("INCOMPLETE")));
        let sum =
            crate::py::strings::py_sum(self.verification_results.iter().map(|v| v.confidence));
        m.insert("avg_confidence".into(), json!(sum / total as f64));
        Value::Object(m)
    }
}
