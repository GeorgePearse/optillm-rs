/// Error types for MARS operations.

use std::error::Error as StdError;
use thiserror::Error;

/// Result type for MARS operations
pub type Result<T> = std::result::Result<T, MarsError>;

/// Error types that can occur during MARS execution
#[derive(Error, Debug)]
pub enum MarsError {
    /// Error occurred in agent execution
    #[error("Agent error: {0}")]
    AgentError(String),

    /// Error during solution verification
    #[error("Verification failed: {0}")]
    VerificationError(String),

    /// Error during solution aggregation
    #[error("Aggregation failed: {0}")]
    AggregationError(String),

    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// No solutions were generated
    #[error("No solutions available")]
    NoSolutions,

    /// No solutions passed verification
    #[error("No verified solutions found")]
    NoVerifiedSolutions,

    /// Error extracting answer from response
    #[error("Answer extraction failed: {0}")]
    AnswerExtractionError(String),

    /// Error communicating with model client
    #[error("Client error: {0}")]
    ClientError(String),

    /// Error in core MARS operations
    #[error("Core error: {0}")]
    CoreError(String),

    /// Operation timed out
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Answer format is invalid
    #[error("Invalid answer format")]
    InvalidAnswerFormat,

    /// Error parsing response
    #[error("Parsing error: {0}")]
    ParsingError(String),

    /// Error extracting strategies from solution
    #[error("Strategy extraction failed: {0}")]
    StrategyExtractionError(String),

    /// Error in coordinator operations
    #[error("Coordinator error: {0}")]
    CoordinatorError(String),
}

impl From<Box<dyn StdError>> for MarsError {
    fn from(err: Box<dyn StdError>) -> Self {
        MarsError::CoreError(err.to_string())
    }
}

impl From<optillm_core::OptillmError> for MarsError {
    fn from(err: optillm_core::OptillmError) -> Self {
        MarsError::CoreError(err.to_string())
    }
}
