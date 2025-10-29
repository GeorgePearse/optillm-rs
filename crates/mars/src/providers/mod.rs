//! Provider management and routing
//!
//! This module handles LLM provider configuration, routing, and model client management.
//! It supports multiple providers (OpenAI, Anthropic, Ollama for local models, etc.)
//! and intelligent routing strategies.

/// Provider routing and selection logic
pub mod router;
/// Provider configuration and routing strategies
pub mod config;
/// Ollama local model provider integration
pub mod ollama;

pub use router::{LLMProvider, ModelClientRouter, ModelStream};
pub use config::{ProviderRoutingConfig, ProviderSpec, RoutingStrategy};
pub use ollama::{OllamaClient, OllamaConfig};
