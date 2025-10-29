//! Provider management and routing
//!
//! This module handles LLM provider configuration, routing, and model client management.
//! It supports multiple providers (OpenAI, Anthropic, etc.) and intelligent routing strategies.

pub mod router;
pub mod config;

pub use router::{LLMProvider, ModelClientRouter, ModelStream};
pub use config::{ProviderRoutingConfig, ProviderSpec, RoutingStrategy};
