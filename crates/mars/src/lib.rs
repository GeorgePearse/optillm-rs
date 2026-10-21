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
// Legacy native-API crate: style lints are allowed so the workspace-wide
// `clippy -D warnings` hook can gate the parity crates.
#![allow(
    clippy::field_reassign_with_default,
    clippy::manual_clamp,
    clippy::unnecessary_sort_by,
    clippy::too_many_arguments,
    clippy::new_without_default,
    clippy::unnecessary_filter_map
)]

// Re-export commonly used types from optillm-core
pub use optillm_core::{
    ContentItem, ModelClient, OptillmError, Optimizer, OptimizerConfig, Prompt, ResponseEvent,
    ResponseItem, TokenUsage,
};

/// Configuration types for MARS system
pub mod config;
/// Compatibility layer for core integration
pub mod core_compat;
/// Error types and error handling
pub mod error;
/// Core type definitions for MARS
pub mod types;

// Re-export MARS-specific types
pub use config::MarsConfig;
pub use error::{MarsError, Result};

/// Core MARS system components including agents, coordinators, and verifiers
pub mod core;
/// LLM provider integrations
pub mod providers;
/// Optimization strategies for solution generation and refinement
pub mod strategies;

// Re-export core components
pub use core::{Agent, Aggregator, MarsCoordinator, StrategyNetwork, Verifier, Workspace};

// Re-export strategy implementations
pub use strategies::{
    AnswerExtractionStrategy,
    // Best-of-N
    BestOfNAggregator,
    BestOfNConfig,
    BestOfNMetadata,
    // CoT Reflection
    CotReflectionAggregator,
    CotReflectionConfig,
    CotReflectionMetadata,
    DialogueState,
    // Diverse Sampling
    DiverseSamplingAggregator,
    DiverseSamplingConfig,
    DiverseSamplingMetadata,
    // LEAP
    LEAPAggregator,
    LEAPConfig,
    LEAPMetadata,
    MCTSConfig,
    MCTSNode,
    Message,
    // MOA
    MoaAggregator,
    MoaMetadata,
    // PVG
    PVGAggregator,
    PVGConfig,
    PVGMetadata,
    // PlanSearch
    PlanSearchAggregator,
    PlanSearchConfig,
    PlanSearchMetadata,
    // RSA
    RSAAggregator,
    RSAConfig,
    RSAMetadata,
    // RTO
    RTOAggregator,
    RTOConfig,
    RTOMetadata,
    // ReRead
    ReReadAggregator,
    ReReadConfig,
    ReReadMetadata,
    ReasoningPath,
    RefinementStrategy,
    SelectionCriterion,
    SelectionMethod,
    SelectionStatistics,
    // Self-Consistency
    SelfConsistencyAggregator,
    SelfConsistencyConfig,
    SelfConsistencyMetadata,
    VotingStatistics,
    VotingStrategy,
    // MCTS
    MCTS,
};

// Re-export provider types
pub use providers::{
    LLMProvider, ModelClientRouter, ModelStream, OllamaClient, OllamaConfig, ProviderRoutingConfig,
    ProviderSpec, RoutingStrategy,
};

/// MARS module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
