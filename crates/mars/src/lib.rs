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

// Organized module structure
pub mod core;
pub mod strategies;
pub mod providers;

// Re-export core components
pub use core::{
    MarsCoordinator, Agent, Workspace, Verifier, Aggregator, StrategyNetwork,
};

// Re-export strategy implementations
pub use strategies::{
    // Best-of-N
    BestOfNAggregator, BestOfNConfig, BestOfNMetadata, SelectionMethod, SelectionStatistics,
    // Self-Consistency
    SelfConsistencyAggregator, SelfConsistencyConfig, SelfConsistencyMetadata,
    AnswerExtractionStrategy, VotingStrategy, ReasoningPath, VotingStatistics,
    // RSA
    RSAAggregator, RSAConfig, RSAMetadata, SelectionCriterion, RefinementStrategy,
    // MCTS
    MCTS, MCTSConfig, MCTSNode, DialogueState, Message,
    // MOA
    MoaAggregator, MoaMetadata,
    // CoT Reflection
    CotReflectionAggregator, CotReflectionConfig, CotReflectionMetadata,
};

// Re-export provider types
pub use providers::{
    LLMProvider, ModelClientRouter, ModelStream,
    ProviderRoutingConfig, ProviderSpec, RoutingStrategy,
};

/// MARS module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
