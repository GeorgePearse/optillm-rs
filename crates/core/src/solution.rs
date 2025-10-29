//! Solution representation for optimization results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::optimizer::OptimizerMetadata;

/// A solution produced by an optimizer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    /// Unique identifier for this solution
    pub id: String,
    /// The reasoning process/chain of thought
    pub reasoning: String,
    /// The final answer
    pub answer: String,
    /// Number of tokens used to generate this solution
    pub token_count: usize,
    /// When this solution was created
    pub created_at: DateTime<Utc>,
    /// Metadata about which optimizer produced this
    pub metadata: OptimizerMetadata,
}

impl Solution {
    /// Create a new solution
    pub fn new(
        reasoning: impl Into<String>,
        answer: impl Into<String>,
        token_count: usize,
        metadata: OptimizerMetadata,
    ) -> Self {
        Self {
            id: format!("solution-{}", Uuid::new_v4()),
            reasoning: reasoning.into(),
            answer: answer.into(),
            token_count,
            created_at: Utc::now(),
            metadata,
        }
    }

    /// Get just the answer
    pub fn get_answer(&self) -> &str {
        &self.answer
    }

    /// Get just the reasoning
    pub fn get_reasoning(&self) -> &str {
        &self.reasoning
    }
}
