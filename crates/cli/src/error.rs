//! Error types for the CLI

use thiserror::Error;

pub type CliResult<T> = Result<T, CliError>;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Tracing initialization error: {0}")]
    TracingInit(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Model client error: {0}")]
    ModelClient(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<tracing_subscriber::filter::ParseError> for CliError {
    fn from(err: tracing_subscriber::filter::ParseError) -> Self {
        CliError::TracingInit(err.to_string())
    }
}
