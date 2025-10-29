/// Provider configuration for multi-model support.
///
/// Manages provider selection, API keys, and routing strategies.

use serde::{Deserialize, Serialize};

/// Specification for a single LLM provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderSpec {
    /// Provider name (e.g., "openai", "anthropic", "groq")
    pub provider: String,

    /// Model identifier (e.g., "gpt-4o", "claude-3-5-sonnet")
    pub model: String,

    /// API key for authentication (can be loaded from env)
    pub api_key: String,

    /// Optional custom base URL for provider
    pub base_url: Option<String>,

    /// Whether this provider is available for multi-agent use
    pub enabled: bool,

    /// Priority for selection (higher = preferred)
    pub priority: usize,
}

impl ProviderSpec {
    /// Create new provider specification with defaults
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
            api_key: String::new(),
            base_url: None,
            enabled: true,
            priority: 0,
        }
    }

    /// Create from environment variable for API key
    pub fn with_env_key(mut self, env_var: &str) -> Self {
        if let Ok(key) = std::env::var(env_var) {
            self.api_key = key;
        }
        self
    }

    /// Set API key
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = key;
        self
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set enabled status
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }

    /// Validate that required fields are set
    pub fn validate(&self) -> Result<(), String> {
        if self.provider.is_empty() {
            return Err("Provider name cannot be empty".to_string());
        }
        if self.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        if self.api_key.is_empty() {
            return Err(format!(
                "API key not set for provider: {}",
                self.provider
            ));
        }
        Ok(())
    }
}

/// Strategy for routing requests to providers
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Always use the primary provider
    Primary,

    /// Round-robin through available providers
    RoundRobin,

    /// Use provider with highest priority
    HighestPriority,

    /// Random selection from enabled providers
    Random,

    /// Use cheapest provider (requires cost data)
    Cheapest,

    /// Use fastest provider (requires latency data)
    Fastest,

    /// Distribute evenly across all providers
    BalanceLoad,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::Primary
    }
}

/// Configuration for multi-provider routing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderRoutingConfig {
    /// Primary provider (fallback if others fail)
    pub primary: ProviderSpec,

    /// Alternative providers for diversity/failover
    pub alternatives: Vec<ProviderSpec>,

    /// Routing strategy for selecting providers
    pub strategy: RoutingStrategy,

    /// Enable fallback on provider failure
    pub enable_fallback: bool,

    /// Maximum retries per provider on failure
    pub max_retries: usize,

    /// Timeout per provider in seconds
    pub timeout_seconds: u64,
}

impl ProviderRoutingConfig {
    /// Create configuration with single provider
    pub fn single(provider: ProviderSpec) -> Self {
        Self {
            primary: provider,
            alternatives: Vec::new(),
            strategy: RoutingStrategy::Primary,
            enable_fallback: true,
            max_retries: 1,
            timeout_seconds: 300,
        }
    }

    /// Create configuration with multiple providers
    pub fn multi(primary: ProviderSpec, alternatives: Vec<ProviderSpec>) -> Self {
        Self {
            primary,
            alternatives,
            strategy: RoutingStrategy::RoundRobin,
            enable_fallback: true,
            max_retries: 2,
            timeout_seconds: 300,
        }
    }

    /// Get all enabled providers
    pub fn get_enabled_providers(&self) -> Vec<&ProviderSpec> {
        let mut providers = vec![&self.primary];
        providers.extend(
            self.alternatives
                .iter()
                .filter(|p| p.enabled),
        );
        providers.retain(|p| p.enabled);
        providers
    }

    /// Validate all provider configurations
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Err(e) = self.primary.validate() {
            errors.push(format!("Primary provider: {}", e));
        }

        for (idx, alt) in self.alternatives.iter().enumerate() {
            if alt.enabled {
                if let Err(e) = alt.validate() {
                    errors.push(format!("Alternative provider {}: {}", idx, e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Set routing strategy
    pub fn with_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Enable/disable fallback
    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.enable_fallback = enabled;
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_spec_creation() {
        let spec = ProviderSpec::new("openai", "gpt-4o");
        assert_eq!(spec.provider, "openai");
        assert_eq!(spec.model, "gpt-4o");
        assert!(spec.enabled);
    }

    #[test]
    fn test_provider_spec_validation() {
        let spec = ProviderSpec::new("openai", "gpt-4o");
        assert!(spec.validate().is_err()); // No API key

        let spec = spec.with_api_key("test-key".to_string());
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_routing_strategy_default() {
        assert_eq!(RoutingStrategy::default(), RoutingStrategy::Primary);
    }

    #[test]
    fn test_provider_routing_config() {
        let primary = ProviderSpec::new("openai", "gpt-4o")
            .with_api_key("key1".to_string());
        let config = ProviderRoutingConfig::single(primary);

        assert_eq!(config.strategy, RoutingStrategy::Primary);
        assert!(config.enable_fallback);
    }

    #[test]
    fn test_multi_provider_routing() {
        let primary = ProviderSpec::new("openai", "gpt-4o")
            .with_api_key("key1".to_string());
        let alt = ProviderSpec::new("anthropic", "claude-3-5-sonnet")
            .with_api_key("key2".to_string());

        let config = ProviderRoutingConfig::multi(primary, vec![alt]);
        assert_eq!(config.strategy, RoutingStrategy::RoundRobin);
        assert_eq!(config.get_enabled_providers().len(), 2);
    }
}
