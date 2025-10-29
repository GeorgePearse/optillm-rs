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
//! use optillm_mars::{MarsCoordinator, MarsConfig};
//! use optillm_core::ModelClient;
//!
//! let config = MarsConfig::default();
//! let coordinator = MarsCoordinator::new(config);
//! let result = coordinator.optimize(query, client).await?;
//! println!("Answer: {}", result.answer);
//! ```

// Re-export commonly used types from optillm-core
pub use optillm_core::{
    ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem, TokenUsage,
    Optimizer, OptimizerConfig, OptillmError,
};

pub mod config;
pub mod error;
pub mod types;
pub mod core_compat;

// Re-export MARS-specific types
pub use error::{MarsError, Result};
pub use config::MarsConfig;

// Core modules
pub mod coordinator;
pub mod agent;
pub mod workspace;
pub mod verifier;
pub mod aggregator;
pub mod strategy;
pub mod prompts;

// New strategy implementations
pub mod mcts;
pub mod moa;
pub mod model_router;
pub mod provider_config;

pub use coordinator::MarsCoordinator;
pub use agent::Agent;
pub use workspace::Workspace;
pub use verifier::Verifier;
pub use aggregator::Aggregator;
pub use strategy::StrategyNetwork;

// Strategy implementations
pub use moa::MoaAggregator;
pub use model_router::{LLMProvider, ModelClientRouter, ModelStream};
pub use provider_config::{ProviderRoutingConfig, ProviderSpec, RoutingStrategy};

/// MARS module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
