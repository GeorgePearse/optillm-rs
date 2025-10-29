# Optimizer Trait

The `Optimizer` trait is the primary extension point for implementing new optimization techniques in optillm-rs.

## Definition

```rust
#[async_trait]
pub trait Optimizer {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution>;

    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
```

## Core Methods

### optimize()

Runs the optimization technique on the given query.

**Parameters:**
- `query`: The problem or question to optimize
- `client`: The LLM provider to use

**Returns:** `Result<Solution>` containing reasoning and answer

**Example:**
```rust
let solution = optimizer.optimize("What is 2+2?", &client).await?;
println!("Answer: {}", solution.answer);
println!("Reasoning: {}", solution.reasoning);
```

### name()

Human-readable name for the optimizer.

```rust
fn name(&self) -> &str {
    "My Optimizer"
}
```

### description()

Description of what the optimizer does.

```rust
fn description(&self) -> &str {
    "Generates solutions using technique X, then verifies with technique Y"
}
```

## Implementation Pattern

```rust
use optillm_core::{Optimizer, Solution, Result};
use async_trait::async_trait;

pub struct MyOptimizer {
    pub temperature: f32,
    pub max_iterations: usize,
}

#[async_trait]
impl Optimizer for MyOptimizer {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // 1. Set up
        let mut token_count = 0;

        // 2. Generate initial solution
        let (initial_reasoning, _) = self
            .generate_solution(query, client)
            .await?;

        // 3. Iterate/refine
        let mut best_answer = String::new();
        for i in 0..self.max_iterations {
            let (reasoning, answer) = self
                .improve_solution(
                    query,
                    &initial_reasoning,
                    client,
                )
                .await?;

            if is_better(&answer, &best_answer) {
                best_answer = answer;
            }
        }

        // 4. Return solution
        Ok(Solution::new(
            self.name().to_string(),
            initial_reasoning,
            best_answer,
            self.temperature,
            token_count,
        ))
    }

    fn name(&self) -> &str {
        "My Optimizer"
    }

    fn description(&self) -> &str {
        "Optimizes solutions through iterative refinement"
    }
}

impl MyOptimizer {
    async fn generate_solution(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<(String, String)> {
        // Stream from LLM and collect response
        Ok((reasoning, answer))
    }

    async fn improve_solution(
        &self,
        query: &str,
        reasoning: &str,
        client: &dyn ModelClient,
    ) -> Result<(String, String)> {
        // Refine based on feedback
        Ok((refined_reasoning, refined_answer))
    }
}
```

## Solution Structure

Return a `Solution` containing:

```rust
pub struct Solution {
    /// ID of agent/optimizer that created this
    pub agent_id: String,

    /// Reasoning chain/explanation
    pub reasoning: String,

    /// Final answer
    pub answer: String,

    /// Temperature used (for tracking)
    pub temperature: f32,

    /// Tokens consumed
    pub token_count: usize,

    /// Verification status (optional)
    pub is_verified: bool,
    pub verification_score: f32,

    /// Phase tracking
    pub phase: GenerationPhase,
}
```

## Common Patterns

### Multi-Agent Approach

```rust
#[async_trait]
impl Optimizer for MultiAgent {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // 1. Create N agents with different temperatures
        let agents: Vec<_> = (0..self.num_agents)
            .map(|i| Agent::new(0.3 + i as f32 * 0.35))
            .collect();

        // 2. Generate solutions in parallel
        let solutions: Vec<_> = futures::future::try_join_all(
            agents.iter().map(|a| a.solve(query, client))
        ).await?;

        // 3. Select best solution
        let best = self.select_best(&solutions);
        Ok(best)
    }
    // ...
}
```

### Verification Pattern

```rust
#[async_trait]
impl Optimizer for Verified {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // 1. Generate solution
        let mut solution = self.generate(query, client).await?;

        // 2. Verify solution
        let score = self.verify(&solution, client).await?;
        solution.verification_score = score;
        solution.is_verified = score > 0.8;

        Ok(solution)
    }
    // ...
}
```

### Iterative Improvement

```rust
#[async_trait]
impl Optimizer for Iterative {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        let mut solution = self.generate(query, client).await?;

        for iteration in 0..self.max_iterations {
            let feedback = self.get_feedback(&solution, client).await?;

            if feedback.should_improve {
                solution = self.improve(
                    &solution,
                    &feedback,
                    client,
                ).await?;
            } else {
                break;
            }
        }

        Ok(solution)
    }
    // ...
}
```

## Error Handling

```rust
pub type Result<T> = std::result::Result<T, OptillmError>;

// Return errors from your optimizer
if config.temperature > 2.0 {
    return Err(OptillmError::InvalidConfiguration(
        "Temperature too high".to_string()
    ));
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockClient;

    impl ModelClient for MockClient {
        fn stream(&self, _: &Prompt) -> /* ... */ {
            // Return mock responses
        }
    }

    #[tokio::test]
    async fn test_optimizer() {
        let optimizer = MyOptimizer::default();
        let client = MockClient;

        let result = optimizer
            .optimize("test query", &client)
            .await;

        assert!(result.is_ok());
    }
}
```

See [Creating New Optimizers](../development/creating-optimizers.md) for more examples.
