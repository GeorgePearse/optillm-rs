# Creating New Optimizers

Guide to implementing a new optimization strategy.

## Overview

All optimizers implement the `Optimizer` trait from `optillm-core`.

## Step-by-Step Guide

### 1. Create Crate Structure

```bash
mkdir -p crates/my-optimizer/src
cd crates/my-optimizer
```

### 2. Create Cargo.toml

```toml
[package]
name = "optillm-my-optimizer"
version = "0.1.0"
edition = "2021"

[dependencies]
optillm-core = { path = "../core" }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 3. Implement Optimizer Trait

Create `src/lib.rs`:

```rust
use optillm_core::{Optimizer, Solution, ModelClient, Result, Prompt, ResponseEvent};
use async_trait::async_trait;

pub struct MyOptimizer {
    // Your configuration
}

impl MyOptimizer {
    pub fn new() -> Self {
        Self {
            // Initialize config
        }
    }
}

#[async_trait]
impl Optimizer for MyOptimizer {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // 1. Create prompt
        let mut prompt = Prompt::default();
        prompt.input = vec![
            // Build prompt structure
        ];

        // 2. Get response from LLM
        let mut stream = client.stream(&prompt);
        let mut response = String::new();

        while let Some(event) = stream.next().await {
            match event? {
                ResponseEvent::OutputTextDelta { delta, .. } => {
                    response.push_str(&delta);
                }
                ResponseEvent::Completed { token_usage } => {
                    // Handle completion
                    break;
                }
                _ => {}
            }
        }

        // 3. Create solution
        let solution = Solution::new(
            "my-optimizer".to_string(),
            "Reasoning here".to_string(),
            response,
            0.7,  // temperature
            0,    // token count
        );

        Ok(solution)
    }

    fn name(&self) -> &str {
        "my-optimizer"
    }

    fn description(&self) -> &str {
        "Description of your optimizer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimizer() {
        let optimizer = MyOptimizer::new();
        // Test with mock client
    }
}
```

### 4. Add to Workspace

Edit root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/core",
    "crates/mars",
    "crates/my-optimizer",  # Add this
]
```

### 5. Write Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let optimizer = MyOptimizer::new();
        assert_eq!(optimizer.name(), "my-optimizer");
    }

    #[tokio::test]
    async fn test_optimization() {
        let optimizer = MyOptimizer::new();
        let mock_client = MockClient::new();

        let result = optimizer.optimize("test query", &mock_client).await;
        assert!(result.is_ok());
    }
}
```

### 6. Create Documentation

Create `crates/my-optimizer/README.md`:

```markdown
# optillm-my-optimizer

## Overview

Brief description of your optimization strategy.

## Usage

```rust
use optillm_my_optimizer::MyOptimizer;

let optimizer = MyOptimizer::new();
let result = optimizer.optimize(query, &client).await?;
```

## Features

- Feature 1
- Feature 2

## Benchmarks

Performance metrics compared to baselines.

## Algorithm

Detailed explanation of the algorithm.
```

### 7. Submit Pull Request

Include:
- Implementation code
- Comprehensive tests
- Documentation with examples
- Benchmark comparisons
- Design decision explanations

## Testing Your Optimizer

### Unit Tests

```bash
cargo test -p optillm-my-optimizer
```

### Integration Tests

```bash
cargo test --all
```

### Performance Testing

```rust
use std::time::Instant;

#[tokio::test]
async fn bench_optimizer() {
    let optimizer = MyOptimizer::new();
    let start = Instant::now();

    let result = optimizer.optimize(query, &client).await;

    println!("Time: {:?}", start.elapsed());
}
```

## Design Patterns

### Pattern 1: Configuration
```rust
pub struct MyOptimizerConfig {
    pub param1: usize,
    pub param2: f32,
}

impl Default for MyOptimizerConfig {
    fn default() -> Self {
        Self {
            param1: 3,
            param2: 0.7,
        }
    }
}
```

### Pattern 2: Builder
```rust
impl MyOptimizer {
    pub fn with_param1(mut self, value: usize) -> Self {
        self.config.param1 = value;
        self
    }
}
```

### Pattern 3: Event Streaming
```rust
pub enum OptimizationEvent {
    Started,
    PhaseCompleted { phase: String },
    Completed { solution_id: String },
}
```

## Common Implementation Patterns

### Multiple Agent Exploration
```rust
let mut solutions = Vec::new();
for temperature in &self.temperatures {
    let solution = self.explore(query, temperature, client).await?;
    solutions.push(solution);
}
```

### Iterative Refinement
```rust
let mut best = initial_solution;
for iteration in 0..max_iterations {
    let improved = self.refine(&best, client).await?;
    if improved.score > best.score {
        best = improved;
    }
}
```

### Strategy Learning
```rust
let mut strategy_network = StrategyNetwork::new();
for solution in &solutions {
    if solution.is_verified {
        strategy_network.register_strategy(
            agent_id,
            description,
            technique,
        );
    }
}
```

## Example: Simple Optimizer

```rust
use optillm_core::{Optimizer, Solution, ModelClient, Result};
use async_trait::async_trait;

pub struct SimpleOptimizer;

#[async_trait]
impl Optimizer for SimpleOptimizer {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // Ask LLM once
        let mut prompt = Prompt::default();
        // ... build prompt

        let mut stream = client.stream(&prompt);
        let mut response = String::new();

        while let Some(event) = stream.next().await {
            if let Ok(ResponseEvent::OutputTextDelta { delta, .. }) = event {
                response.push_str(&delta);
            }
        }

        Ok(Solution::new(
            "simple".to_string(),
            "Direct response".to_string(),
            response,
            0.7,
            0,
        ))
    }

    fn name(&self) -> &str { "simple" }
    fn description(&self) -> &str { "Simple single-pass optimizer" }
}
```

## Next Steps

1. Implement your optimizer
2. Write comprehensive tests
3. Add documentation
4. Submit for review
5. Iterate based on feedback

---

Questions? See [Contributing Guide](contributing.md)
