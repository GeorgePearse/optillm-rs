# API Reference: optillm-mars

Complete API documentation for the optillm-mars library.

## Main Components

### MarsCoordinator

```rust
pub struct MarsCoordinator {
    config: MarsConfig,
}

impl MarsCoordinator {
    pub fn new(config: MarsConfig) -> Self;
}

#[async_trait]
impl Optimizer for MarsCoordinator {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution>;
}
```

### Agent

```rust
pub struct Agent {
    pub id: String,
    pub temperature: f32,
}

impl Agent {
    pub fn new(temperature: f32) -> Self;

    pub async fn generate_solution_with_client(
        &self,
        query: &str,
        use_thinking_tags: bool,
        client: &dyn ModelClient,
    ) -> Result<Solution>;

    pub async fn verify_solution_with_client(
        &self,
        solution: &Solution,
        client: &dyn ModelClient,
    ) -> Result<f32>;

    pub async fn improve_solution_with_client(
        &self,
        solution: &Solution,
        feedback: &str,
        use_thinking_tags: bool,
        client: &dyn ModelClient,
    ) -> Result<Solution>;

    pub async fn extract_strategies_with_client(
        &self,
        solution: &Solution,
        client: &dyn ModelClient,
    ) -> Result<Vec<String>>;
}
```

## Configuration Types

### MarsConfig

```rust
pub struct MarsConfig {
    pub num_agents: usize,
    pub temperatures: [f32; 3],
    pub max_tokens: usize,
    pub num_verification_rounds: usize,
    pub verification_threshold: f32,
    pub use_thinking_tags: bool,
    pub max_improvement_iterations: usize,
}

impl Default for MarsConfig {
    fn default() -> Self;
}
```

### Builder Methods

```rust
impl MarsConfig {
    pub fn with_num_agents(mut self, n: usize) -> Self;
    pub fn with_max_tokens(mut self, n: usize) -> Self;
    pub fn with_verification_threshold(mut self, t: f32) -> Self;
    pub fn with_thinking_tags(mut self, enabled: bool) -> Self;
}
```

## Core Types

### MarsEvent

```rust
pub enum MarsEvent {
    ExplorationStarted { num_agents: usize },
    SolutionGenerated { solution_id: String, agent_id: String },
    SolutionVerified { solution_id: String, is_correct: bool, score: f32 },
    // ... more events
}
```

### Solution

See [optillm-core API Reference](core.md) for full `Solution` definition.

## Aggregators

### BestOfNAggregator

```rust
pub struct BestOfNAggregator;

impl BestOfNAggregator {
    pub async fn run_best_of_n(
        query: &str,
        system_prompt: &str,
        config: BestOfNConfig,
        client: &dyn ModelClient,
    ) -> Result<(Solution, BestOfNMetadata)>;
}
```

### Other Aggregators

- `SelfConsistencyAggregator`
- `RSAAggregator`
- `MoaAggregator`
- `CotReflectionAggregator`
- `RTOAggregator`
- `PVGAggregator`
- `LEAPAggregator`
- `PlanSearchAggregator`
- `ReReadAggregator`
- `DiverseSamplingAggregator`

## Complete API

For complete API documentation with all methods and fields, run:

```bash
cargo doc -p optillm-mars --open
```

See [MARS Documentation](../mars/overview.md) for usage guides.
