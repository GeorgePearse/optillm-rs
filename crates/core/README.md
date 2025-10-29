# optillm-core

Core library providing shared interfaces, types, and traits for all optillm optimization implementations.

## Overview

`optillm-core` defines the common abstractions that all optillm implementations (MARS, Best-of-N, Monte Carlo Tree Search, etc.) must implement. This ensures compatibility and interoperability between different optimization techniques.

## Core Abstractions

### `ModelClient` Trait

Interface for communicating with language models. Implementations handle actual LLM calls.

```rust
pub trait ModelClient: Send + Sync {
    fn stream(&self, prompt: &Prompt)
        -> Pin<Box<dyn Stream<Item = Result<ResponseEvent>> + Send>>;
}
```

### `Optimizer` Trait

Interface all optimization implementations must implement.

```rust
#[async_trait]
pub trait Optimizer: Send + Sync {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution>;

    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
```

### `Solution` Struct

Represents a complete solution with reasoning and answer.

```rust
pub struct Solution {
    pub id: String,
    pub reasoning: String,
    pub answer: String,
    pub token_count: usize,
    pub created_at: DateTime<Utc>,
    pub metadata: OptimizerMetadata,
}
```

### `Prompt` and `ResponseEvent`

Unified request/response types for LLM communication:

- `Prompt`: Contains messages, system instructions, and optional log tags
- `ResponseEvent`: Stream of text deltas and completion events with token counts

## Type Hierarchy

```
optillm-core/
├── client.rs       # ModelClient trait, Prompt, ResponseEvent
├── optimizer.rs    # Optimizer trait, OptimizerConfig
├── solution.rs     # Solution, OptimizerMetadata
└── error.rs        # OptillmError, Result
```

## Usage

All implementations depend on optillm-core:

```toml
[dependencies]
optillm-core = { path = "../core" }
```

And implement the `Optimizer` trait:

```rust
use optillm_core::*;

pub struct MyOptimizer {
    config: MyConfig,
}

#[async_trait]
impl Optimizer for MyOptimizer {
    async fn optimize(&self, query: &str, client: &dyn ModelClient) -> Result<Solution> {
        // Your implementation
    }

    fn name(&self) -> &str { "my-optimizer" }
    fn description(&self) -> &str { "..." }
}
```

## Error Handling

Provides unified error types through `OptillmError`:

- `ClientError`: LLM communication failures
- `InvalidConfiguration`: Config validation errors
- `NoSolutions`: When optimization fails to produce solutions
- `ParsingError`: When parsing model outputs
- `AnswerExtractionError`: When extracting answers from reasoning
- `Timeout`: Operation timeout
- `OptimizerError`: Implementation-specific errors

## Version

Versions are managed at workspace level in root `Cargo.toml`.

## License

MIT
