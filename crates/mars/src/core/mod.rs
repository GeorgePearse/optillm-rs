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
