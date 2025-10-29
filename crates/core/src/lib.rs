#![warn(missing_docs)]

//! Core types, traits, and interfaces for optillm optimization implementations.
//!
//! This crate provides the common abstractions that all optillm implementations
//! (MARS, Monte Carlo Tree Search, Best-of-N, etc.) should implement.

pub mod client;
pub mod error;
pub mod optimizer;
pub mod solution;

pub use client::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem, TokenUsage};
pub use error::{OptillmError, Result};
pub use optimizer::{Optimizer, OptimizerConfig};
pub use solution::Solution;

/// Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
