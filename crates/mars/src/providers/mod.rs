//! Provider management and routing
//!
//! This module handles LLM provider configuration, routing, and model client management.
//! It supports multiple providers (OpenAI, Anthropic, Ollama for local models, etc.)
//! and intelligent routing strategies.

/// Provider configuration and routing strategies
pub mod config;
/// Ollama local model provider integration
pub mod ollama;
/// Provider routing and selection logic
pub mod router;

pub use config::{ProviderRoutingConfig, ProviderSpec, RoutingStrategy};
pub use ollama::{OllamaClient, OllamaConfig};
pub use router::{LLMProvider, ModelClientRouter, ModelStream};
