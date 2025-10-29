# Integration with Code (Codex Fork)

This guide explains how to integrate optillm-rs optimization strategies into the [code](https://github.com/coohom/code) project.

## Overview

optillm-rs is specifically designed to seamlessly integrate with code's reasoning and agent systems, providing:

- **Advanced Optimization**: MARS and other strategies to improve reasoning quality
- **Multi-Provider Support**: Unified interface for all LLM providers via litellm-rs
- **Type Safety**: Rust's safety guarantees in production systems
- **Performance**: High-speed optimization without Python overhead

## Architecture

```
code (Codex fork)
    ↓
[Reasoning Pipeline]
    ↓
[optillm-rs Coordinator]
    ├─ MARS Optimizer
    ├─ MOA Strategy
    └─ MCTS Strategy
    ↓
[LiteLLM Router]
    ├─ OpenAI
    ├─ Anthropic
    ├─ Google
    └─ Other Providers
```

## Setup Steps

### 1. Add Dependency

In code's Cargo.toml:

```toml
[dependencies]
optillm-core = { path = "../optillm-rs/crates/core" }
optillm-mars = { path = "../optillm-rs/crates/mars" }

# For LiteLLM support (future)
# litellm = "0.1"  # When published
```

### 2. Create Provider Bridge

Create `src/optillm_bridge.rs` in code:

```rust
use optillm_core::{ModelClient, Prompt, ResponseEvent, OptillmError, TokenUsage};
use std::pin::Pin;
use futures::Stream;

/// Bridge between code's ModelClient and optillm's ModelClient trait
pub struct CodeModelClientBridge {
    inner: Arc<dyn YourModelClient>,  // Your existing client
}

impl CodeModelClientBridge {
    pub fn new(client: Arc<dyn YourModelClient>) -> Self {
        Self { inner: client }
    }
}

impl ModelClient for CodeModelClientBridge {
    fn stream(&self, prompt: &Prompt)
        -> Pin<Box<dyn Stream<Item = Result<ResponseEvent, OptillmError>> + Send>> {
        // Adapt code's response format to optillm's ResponseEvent
        // This bridges the two systems transparently

        let inner = self.inner.clone();
        let prompt = prompt.clone();

        Box::pin(async_stream::stream! {
            // Stream responses from code's client
            // Convert to ResponseEvent format
        })
    }
}
```

### 3. Initialize Coordinator

In your coordinator or agent init code:

```rust
use optillm_mars::{MarsCoordinator, MarsConfig};
use crate::optillm_bridge::CodeModelClientBridge;

pub struct CodeCoordinator {
    model_client: Arc<dyn CodeModelClient>,
    optillm: MarsCoordinator,
}

impl CodeCoordinator {
    pub fn new(model_client: Arc<dyn CodeModelClient>) -> Self {
        // Configure MARS optimizer
        let config = MarsConfig {
            num_agents: 5,
            temperatures: vec![0.3, 0.6, 0.9, 1.0, 1.2],
            max_iterations: 2,
            verification_threshold: 0.75,
            enable_strategy_learning: true,
            ..Default::default()
        };

        let optillm = MarsCoordinator::new(config);

        Self {
            model_client,
            optillm,
        }
    }

    /// Optimize a reasoning query using MARS
    pub async fn optimize_reasoning(
        &self,
        query: &str,
    ) -> Result<OptimizedResult, Error> {
        let bridge = CodeModelClientBridge::new(self.model_client.clone());

        let result = self.optillm.optimize(query, &bridge).await?;

        Ok(OptimizedResult {
            answer: result.answer,
            reasoning: result.reasoning,
            confidence: result.final_solution_id,
            tokens_used: result.total_tokens,
            iterations: result.iterations,
        })
    }
}
```

### 4. Integrate with Code's Pipeline

In code's agent or reasoning module:

```rust
// Traditional reasoning
let response = agent.reason(query).await?;

// Or optimized reasoning using MARS
let coordinator = CodeCoordinator::new(model_client.clone());
let optimized = coordinator.optimize_reasoning(query).await?;

// Use optimized response
println!("Answer: {}", optimized.answer);
println!("Reasoning Quality: {}", optimized.confidence);
println!("Tokens Used: {}", optimized.tokens_used);
```

## LiteLLM Integration

### Setting Up Multi-Provider Support

```rust
use optillm_mars::provider_config::{ProviderSpec, ProviderRoutingConfig, RoutingStrategy};

fn create_provider_config() -> ProviderRoutingConfig {
    // Primary provider
    let openai = ProviderSpec::new("openai", "gpt-4o")
        .with_env_key("OPENAI_API_KEY")
        .with_priority(1);

    // Fallback providers
    let anthropic = ProviderSpec::new("anthropic", "claude-3-5-sonnet")
        .with_env_key("ANTHROPIC_API_KEY")
        .with_priority(2);

    let groq = ProviderSpec::new("groq", "mixtral-8x7b-32k")
        .with_env_key("GROQ_API_KEY")
        .with_priority(3);

    // Configure routing strategy
    ProviderRoutingConfig::multi(openai, vec![anthropic, groq])
        .with_strategy(RoutingStrategy::RoundRobin)
        .with_fallback(true)
        .with_max_retries(2)
        .with_timeout(300)
}
```

### Provider Routing Strategies

#### 1. Primary (Default)
Always use the primary provider, fallback on error:

```rust
config.with_strategy(RoutingStrategy::Primary)
```

#### 2. Round Robin
Distribute across providers:

```rust
config.with_strategy(RoutingStrategy::RoundRobin)
```

#### 3. Highest Priority
Use provider with highest priority:

```rust
config.with_strategy(RoutingStrategy::HighestPriority)
```

#### 4. Random
Random selection for load distribution:

```rust
config.with_strategy(RoutingStrategy::Random)
```

#### 5. Cost-Aware
Use cheapest provider (requires cost data):

```rust
config.with_strategy(RoutingStrategy::Cheapest)
```

## Strategy Selection for Code Tasks

### For Code Generation/Analysis
Use **MARS** for maximum quality:
```rust
let config = MarsConfig {
    num_agents: 7,
    temperatures: vec![0.1, 0.3, 0.5, 0.7, 0.9, 1.1, 1.3],
    max_iterations: 3,
    verification_threshold: 0.85,
    ..Default::default()
};
```

### For Quick Responses
Use **MOA** for balanced quality/cost:
```rust
let (solution, metadata) = MoaAggregator::run_moa(
    query,
    system_prompt,
    3,  // 3 completions
    true,
    &bridge,
).await?;
```

### For Interactive Mode
Use **MCTS** for low-cost exploration:
```rust
let config = MCTSConfig {
    simulation_depth: 2,
    num_simulations: 10,
    num_actions: 3,
    ..Default::default()
};
```

## Monitoring and Observability

### Track Performance Metrics

```rust
pub struct OptimizationMetrics {
    pub query: String,
    pub total_tokens: usize,
    pub latency: Duration,
    pub quality_score: f32,
    pub provider_used: String,
    pub strategy: String,
    pub iterations: usize,
}

impl CodeCoordinator {
    pub async fn optimize_with_metrics(
        &self,
        query: &str,
    ) -> Result<(OptimizedResult, OptimizationMetrics), Error> {
        let start = Instant::now();
        let result = self.optimize_reasoning(query).await?;

        let metrics = OptimizationMetrics {
            query: query.to_string(),
            total_tokens: result.tokens_used,
            latency: start.elapsed(),
            quality_score: result.confidence,
            provider_used: "openai".to_string(),
            strategy: "mars".to_string(),
            iterations: result.iterations,
        };

        Ok((result, metrics))
    }
}
```

### Logging

```rust
use log::{info, debug, warn};

pub async fn optimize_with_logging(
    &self,
    query: &str,
) -> Result<OptimizedResult, Error> {
    info!("Starting MARS optimization for query: {}", query);

    let start = Instant::now();
    let result = self.optimize_reasoning(query).await?;
    let elapsed = start.elapsed();

    info!(
        "Optimization complete: {}ms, {} tokens, {} iterations",
        elapsed.as_millis(),
        result.tokens_used,
        result.iterations
    );

    debug!("Answer: {}", result.answer);
    debug!("Reasoning: {}", result.reasoning);

    Ok(result)
}
```

## Error Handling

```rust
use optillm_core::MarsError;

pub async fn safe_optimize(
    &self,
    query: &str,
) -> Result<OptimizedResult, OptimizationError> {
    match self.optimize_reasoning(query).await {
        Ok(result) => {
            // Verify result quality
            if result.confidence < 0.5 {
                warn!("Low confidence result: {}", result.confidence);
            }
            Ok(result)
        }
        Err(e) => {
            warn!("Optimization failed: {}", e);

            // Fall back to non-optimized path
            let fallback = self.agent.reason(query).await?;
            Ok(OptimizedResult::from_fallback(fallback))
        }
    }
}
```

## Cost Management

### Token Budget

```rust
let config = MarsConfig {
    token_budget: 5000,  // Maximum tokens per query
    ..Default::default()
};
```

### Token Tracking

```rust
pub struct TokenBudget {
    pub budget_per_query: usize,
    pub daily_limit: usize,
    pub current_usage: AtomicUsize,
}

impl TokenBudget {
    pub fn can_optimize(&self, estimated_tokens: usize) -> bool {
        self.current_usage.load(Ordering::Relaxed) + estimated_tokens < self.daily_limit
    }

    pub fn record_usage(&self, tokens: usize) {
        self.current_usage.fetch_add(tokens, Ordering::Relaxed);
    }
}
```

### Cost Reporting

```rust
pub struct CostReport {
    pub total_tokens: usize,
    pub estimated_cost_usd: f64,
    pub by_strategy: HashMap<String, f64>,
    pub by_provider: HashMap<String, f64>,
}
```

## Testing

### Mock Integration

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockModelClient;

    #[async_trait]
    impl ModelClient for MockModelClient {
        fn stream(&self, _prompt: &Prompt)
            -> Pin<Box<dyn Stream<Item = Result<ResponseEvent, OptillmError>> + Send>> {
            Box::pin(async_stream::stream! {
                yield Ok(ResponseEvent::OutputTextDelta {
                    delta: "Mock response".to_string(),
                    index: 0,
                });
                yield Ok(ResponseEvent::Completed {
                    token_usage: TokenUsage {
                        input_tokens: 10,
                        output_tokens: 5,
                        total_tokens: 15,
                    },
                });
            })
        }
    }

    #[tokio::test]
    async fn test_optimization_integration() {
        let client = Arc::new(MockModelClient);
        let coordinator = CodeCoordinator::new(client);

        let result = coordinator.optimize_reasoning("test query").await;
        assert!(result.is_ok());
    }
}
```

## Performance Considerations

### Latency Optimization
```rust
let config = MarsConfig {
    num_agents: 3,           // Fewer agents = faster
    max_iterations: 1,       // Single pass
    parallel_exploration: true,  // Enable parallelism
    ..Default::default()
};
```

### Cost Optimization
```rust
let config = MarsConfig {
    num_agents: 2,
    token_budget: 2000,
    max_iterations: 1,
    ..Default::default()
};
```

### Quality Optimization
```rust
let config = MarsConfig {
    num_agents: 7,
    token_budget: 10000,
    max_iterations: 3,
    verification_threshold: 0.9,
    enable_strategy_learning: true,
    ..Default::default()
};
```

## Troubleshooting

### Provider Connection Issues
```rust
// Enable debug logging
env::set_var("RUST_LOG", "debug");
env_logger::init();

// Check provider availability
if let Err(e) = bridge.test_connection().await {
    warn!("Provider unavailable: {}", e);
    // Fall back to primary provider
}
```

### Low Quality Results
1. Increase `num_agents`
2. Expand temperature range
3. Improve system prompt
4. Increase `max_iterations`
5. Higher `token_budget`

### High Token Usage
1. Reduce `num_agents`
2. Set `token_budget`
3. Decrease `max_iterations`
4. Use faster models

## Next Steps

1. Create the provider bridge in your code project
2. Initialize MARS coordinator
3. Integrate with your agent/reasoning pipeline
4. Set up monitoring and logging
5. Test with sample queries
6. Configure provider routing for your needs
7. Monitor costs and quality metrics

## Support

For issues or questions:
- Check [FAQ](faq.md)
- Review [examples](getting-started/examples.md)
- See [MARS documentation](mars/overview.md)
- Check code's integration tests
