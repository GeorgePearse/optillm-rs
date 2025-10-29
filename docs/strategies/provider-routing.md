# Provider Routing

Provider routing allows intelligent selection and fallback across multiple LLM providers.

## Overview

Provider routing enables:

- **Multi-Provider**: Use multiple LLM providers simultaneously
- **Cost Optimization**: Route to cheapest provider
- **Latency Optimization**: Route to fastest provider
- **Failover**: Automatic fallback on errors
- **Load Balancing**: Distribute across providers

## Configuration

```rust
pub struct ProviderRoutingConfig {
    pub providers: Vec<ProviderSpec>,
    pub strategy: RoutingStrategy,
    pub fallback_enabled: bool,
    pub max_retries: usize,
}

pub enum RoutingStrategy {
    RoundRobin,
    LeastLatency,
    LowestCost,
    Random,
}
```

## Usage

```rust
let openai = ProviderSpec::new("openai", "gpt-4o")
    .with_priority(1);

let anthropic = ProviderSpec::new("anthropic", "claude-3-5-sonnet")
    .with_priority(2);

let config = ProviderRoutingConfig::new(openai, vec![anthropic])
    .with_strategy(RoutingStrategy::RoundRobin);

let router = ModelClientRouter::new(config)?;
```

## Routing Strategies

### RoundRobin
Alternate between providers:

```
Request 1 → OpenAI
Request 2 → Anthropic
Request 3 → OpenAI
```

### LeastLatency
Use fastest responding provider:

```
OpenAI: 200ms
Anthropic: 150ms (selected)
```

### LowestCost
Use cheapest provider:

```
GPT-4: $0.03/1K tokens
Claude: $0.01/1K tokens (selected)
```

### Random
Random provider selection.

## Failover

Automatic fallback on errors:

```
Request → OpenAI (fails)
       → Anthropic (fallback, succeeds)
```

Configure with:

```rust
config.fallback_enabled = true;
config.max_retries = 3;
```

## Load Balancing

Distribute across providers to avoid rate limits:

```
10 concurrent requests
├─ 5 to OpenAI (50%)
└─ 5 to Anthropic (50%)
```

## Cost Tracking

Monitor spending across providers:

```rust
let usage = router.get_usage_stats();
println!("OpenAI: {} tokens, ${}", 
    usage.openai_tokens, 
    usage.openai_cost);
```

## Implementation Patterns

See [MARS Configuration](../mars/configuration.md) for integration examples.
