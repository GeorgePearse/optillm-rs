/// Error types for MARS operations.

use code_core::error::CodexErr;
use thiserror::Error;

/// Result type for MARS operations
pub type Result<T> = std::result::Result<T, MarsError>;

/// Error types that can occur during MARS execution
#[derive(Error, Debug)]
pub enum MarsError {
    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Verification failed: {0}")]
    VerificationError(String),

    #[error("Aggregation failed: {0}")]
    AggregationError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("No solutions available")]
    NoSolutions,

    #[error("No verified solutions found")]
    NoVerifiedSolutions,

    #[error("Answer extraction failed: {0}")]
    AnswerExtractionError(String),

    #[error("Client error: {0}")]
    ClientError(String),

    #[error("Core error: {0}")]
    CoreError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid answer format")]
    InvalidAnswerFormat,

    #[error("Parsing error: {0}")]
    ParsingError(String),

    #[error("Strategy extraction failed: {0}")]
    StrategyExtractionError(String),

    #[error("Coordinator error: {0}")]
    CoordinatorError(String),
}

// Implement conversion from code_core's CodexErr
impl From<CodexErr> for MarsError {
    fn from(err: CodexErr) -> Self {
        MarsError::CoreError(err.to_string())
    }
}
