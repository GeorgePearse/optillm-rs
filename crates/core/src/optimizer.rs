//! Core optimizer trait and configuration.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{Result, Solution, ModelClient};

/// Base configuration trait for all optimizers
pub trait OptimizerConfig: Clone + Send + Sync {
    /// Validate the configuration
    fn validate(&self) -> Result<()>;
}

/// Core optimizer trait that all implementations must implement
#[async_trait]
pub trait Optimizer: Send + Sync {
    /// Run the optimizer on a query and return the best solution
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution>;

    /// Get the name of this optimizer implementation
    fn name(&self) -> &str;

    /// Get a description of this optimizer
    fn description(&self) -> &str;
}

/// Base solution metadata that all optimizers track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerMetadata {
    /// Name of the optimizer that produced this result
    pub optimizer_name: String,
    /// Number of iterations or attempts made
    pub iterations: usize,
    /// Total tokens used
    pub total_tokens: usize,
    /// Time taken in milliseconds
    pub duration_ms: u64,
}

impl OptimizerMetadata {
    /// Create new optimizer metadata
    pub fn new(optimizer_name: impl Into<String>) -> Self {
        Self {
            optimizer_name: optimizer_name.into(),
            iterations: 0,
            total_tokens: 0,
            duration_ms: 0,
        }
    }
}
