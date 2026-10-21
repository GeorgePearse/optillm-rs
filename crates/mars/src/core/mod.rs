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

/// Individual reasoning agents with configurable parameters
pub mod agent;
/// Aggregation of multiple solutions
pub mod aggregator;
/// Main coordinator for orchestrating multi-agent reasoning
pub mod coordinator;
/// Predefined system prompts and templates
pub mod prompts;
/// Strategy network for reasoning approach selection
pub mod strategy;
/// Verification module for cross-validating solutions
pub mod verifier;
/// Workspace for managing shared state and results
pub mod workspace;

pub use agent::Agent;
pub use aggregator::Aggregator;
pub use coordinator::MarsCoordinator;
pub use strategy::StrategyNetwork;
pub use verifier::Verifier;
pub use workspace::Workspace;
