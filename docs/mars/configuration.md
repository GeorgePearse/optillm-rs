# MARS Configuration

This guide covers configuring the MARS (Multi-Agent Reasoning System) optimizer.

## MarsConfig

The main configuration struct for MARS:

```rust
pub struct MarsConfig {
    /// Number of agents to use
    pub num_agents: usize,

    /// Temperature range for agents [low, medium, high]
    pub temperatures: [f32; 3],

    /// Maximum tokens per generation
    pub max_tokens: usize,

    /// Number of verification rounds
    pub num_verification_rounds: usize,

    /// Verification score threshold for passing
    pub verification_threshold: f32,

    /// Enable thinking tags for reasoning
    pub use_thinking_tags: bool,

    /// Maximum improvement iterations
    pub max_improvement_iterations: usize,
}
```

## Default Configuration

```rust
impl Default for MarsConfig {
    fn default() -> Self {
        Self {
            num_agents: 3,
            temperatures: [0.3, 0.6, 1.0],
            max_tokens: 4096,
            num_verification_rounds: 2,
            verification_threshold: 0.7,
            use_thinking_tags: true,
            max_improvement_iterations: 2,
        }
    }
}
```

## Builder Pattern

```rust
let config = MarsConfig::default()
    .with_num_agents(5)
    .with_max_tokens(8192)
    .with_verification_threshold(0.8);
```

## Configuration Options

### num_agents

Number of agents to deploy:

```rust
// Default: 3 agents (low, medium, high temperature)
config.num_agents = 5;  // 5 diverse agents
```

**Tradeoff:**
- More agents = more diverse solutions
- Fewer agents = faster execution

### temperatures

Temperature values for each agent:

```rust
// Default: [0.3, 0.6, 1.0]
config.temperatures = [0.1, 0.5, 0.9];  // More extreme range
```

**Guidelines:**
- 0.0-0.3: Deterministic, focused
- 0.3-0.7: Balanced, diverse
- 0.7-1.0: Creative, exploratory

### max_tokens

Maximum tokens per LLM call:

```rust
config.max_tokens = 8192;  // For longer reasoning
```

### use_thinking_tags

Enable Claude's thinking tags for extended reasoning:

```rust
config.use_thinking_tags = true;  // <think>...</think>
```

## Creating a MarsCoordinator

```rust
use optillm_mars::MarsCoordinator;

let config = MarsConfig::default();
let coordinator = MarsCoordinator::new(config);

let result = coordinator.optimize(query, &client).await?;
```

## Advanced Tuning

### For Complex Problems

```rust
let config = MarsConfig {
    num_agents: 7,
    temperatures: [0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4],
    max_tokens: 8192,
    max_improvement_iterations: 3,
    verification_threshold: 0.8,
    ..Default::default()
};
```

### For Speed

```rust
let config = MarsConfig {
    num_agents: 2,
    temperatures: [0.3, 1.0],
    max_tokens: 2048,
    num_verification_rounds: 1,
    max_improvement_iterations: 1,
    ..Default::default()
};
```

### For Quality

```rust
let config = MarsConfig {
    num_agents: 5,
    temperatures: [0.1, 0.3, 0.6, 0.9, 1.2],
    max_tokens: 8192,
    num_verification_rounds: 3,
    max_improvement_iterations: 3,
    verification_threshold: 0.9,
    use_thinking_tags: true,
    ..Default::default()
};
```

## Environment-Specific Configs

### Development

```rust
let config = MarsConfig {
    num_agents: 2,
    max_tokens: 1024,
    ..Default::default()
};
```

### Production

```rust
let config = MarsConfig {
    num_agents: 5,
    max_tokens: 8192,
    verification_threshold: 0.85,
    max_improvement_iterations: 2,
    ..Default::default()
};
```

## Configuration Validation

```rust
impl MarsConfig {
    pub fn validate(&self) -> Result<()> {
        if self.num_agents == 0 {
            return Err("num_agents must be > 0".into());
        }
        if self.verification_threshold > 1.0 || self.verification_threshold < 0.0 {
            return Err("verification_threshold must be between 0 and 1".into());
        }
        Ok(())
    }
}
```

## Runtime Configuration Changes

```rust
let mut config = MarsConfig::default();

// Adjust based on model capabilities
if model_has_reasoning {
    config.use_thinking_tags = true;
    config.max_tokens = 8192;
}

// Adjust based on latency requirements
if latency_critical {
    config.num_agents = 2;
    config.max_improvement_iterations = 1;
}
```

See [MARS Overview](overview.md) for system architecture details.
