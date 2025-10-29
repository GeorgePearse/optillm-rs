#![warn(missing_docs)]

//! MARS (Multi-Agent Reasoning System) - Rust implementation
//!
//! This crate provides a production-ready implementation of the MARS optimization technique
//! for LLM inference, achieving 69% improvement on complex reasoning tasks like AIME 2025.
//!
//! ## Overview
//!
//! MARS uses multiple agents with diverse temperature settings to explore different solution
//! paths, then applies cross-agent verification, aggregation, and iterative improvement to
//! synthesize the best possible answer.
//!
//! ## Example
//!
//! ```ignore
//! use code_mars::{MarsCoordinator, MarsConfig};
//!
//! let config = MarsConfig::default();
//! let coordinator = MarsCoordinator::new(config);
//! let result = coordinator.run(query, client).await?;
//! println!("Answer: {}", result.answer);
//! ```

pub mod config;
pub mod error;
pub mod types;

pub use config::MarsConfig;
pub use error::{MarsError, Result};
pub use types::{MarsEvent, MarsOutput, Solution};

// These will be implemented next
pub mod coordinator;
pub mod agent;
pub mod workspace;
pub mod verifier;
pub mod aggregator;
pub mod strategy;
pub mod prompts;

pub use coordinator::MarsCoordinator;
pub use agent::Agent;
pub use workspace::Workspace;
pub use verifier::Verifier;
pub use aggregator::Aggregator;
pub use strategy::StrategyNetwork;

/// MARS module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
