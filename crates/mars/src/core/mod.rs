//! Core MARS system components
//!
//! This module contains the fundamental building blocks of the MARS system:
//! - Coordinator: orchestrates the entire multi-agent reasoning process
//! - Agent: individual reasoning agents with diverse settings
//! - Workspace: manages shared state and intermediate results
//! - Verifier: cross-validates agent outputs
//! - Aggregator: combines and synthesizes agent responses
//! - Strategy: defines the reasoning strategy network
//! - Prompts: system prompts and templates

/// Main coordinator for orchestrating multi-agent reasoning
pub mod coordinator;
/// Individual reasoning agents with configurable parameters
pub mod agent;
/// Workspace for managing shared state and results
pub mod workspace;
/// Verification module for cross-validating solutions
pub mod verifier;
/// Aggregation of multiple solutions
pub mod aggregator;
/// Strategy network for reasoning approach selection
pub mod strategy;
/// Predefined system prompts and templates
pub mod prompts;

pub use coordinator::MarsCoordinator;
pub use agent::Agent;
pub use workspace::Workspace;
pub use verifier::Verifier;
pub use aggregator::Aggregator;
pub use strategy::StrategyNetwork;
