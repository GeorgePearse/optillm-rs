//! Error types for optillm operations.

use std::error::Error as StdError;
use thiserror::Error;

/// Result type for optillm operations
pub type Result<T> = std::result::Result<T, OptillmError>;

/// Error types that can occur during optillm optimization
#[derive(Error, Debug)]
pub enum OptillmError {
    /// Error from the model client
    #[error("Client error: {0}")]
    ClientError(String),

    /// Configuration error
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// No solutions were generated
    #[error("No solutions available")]
    NoSolutions,

    /// Parsing or extraction failed
    #[error("Parsing error: {0}")]
    ParsingError(String),

    /// Answer extraction failed
    #[error("Answer extraction failed: {0}")]
    AnswerExtractionError(String),

    /// Timeout during operation
    #[error("Timeout: {0}")]
    Timeout(String),

    /// General optimizer error
    #[error("Optimizer error: {0}")]
    OptimizerError(String),
}

impl From<Box<dyn StdError>> for OptillmError {
    fn from(err: Box<dyn StdError>) -> Self {
        OptillmError::ClientError(err.to_string())
    }
}
