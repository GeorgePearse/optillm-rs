# optillm-core Introduction

`optillm-core` is the foundational library that provides the shared traits, types, and interfaces used by all OptimLLM implementations.

## What is optillm-core?

It provides:

- **Traits**: Abstract interfaces for extending functionality
- **Types**: Common data structures used across implementations
- **Error Handling**: Unified error types
- **Utilities**: Helper functions and macros

## Key Components

### Traits

- **`Optimizer`**: Interface for optimization techniques
- **`ModelClient`**: Interface for LLM provider communication

### Types

- **`Solution`**: Result from optimization containing reasoning and answer
- **`Prompt`**: Request structure for LLM calls
- **`ResponseEvent`**: Events streamed from LLM responses
- **`TokenUsage`**: Token consumption tracking

### Error Types

- **`OptillmError`**: Comprehensive error enum
- **`Result<T>`**: Type alias for `Result<T, OptillmError>`

## Dependencies

- `tokio`: Async runtime
- `async-trait`: Async trait support
- `serde`: Serialization/deserialization
- `thiserror`: Error handling

## When to Use optillm-core

Use `optillm-core` when:

- Creating a new optimization technique
- Implementing a new LLM provider
- Building on top of OptimLLM functionality
- Integrating OptimLLM into your system

## Integration Pattern

```rust
// Add dependency
[dependencies]
optillm-core = { path = "../core" }

// Import types
use optillm_core::{
    Optimizer, ModelClient, Solution, Prompt,
    ResponseEvent, Result,
};

// Implement traits
#[async_trait]
impl Optimizer for MyOptimizer { }

// Use in your code
let result = my_optimizer.optimize(query, client).await?;
```

## Next Steps

- Read [ModelClient](model-client.md) for LLM integration
- Read [Optimizer Trait](optimizer-trait.md) for creating optimizers
- Check [Types](types.md) for data structure details
- Review [Error Handling](error-handling.md) for error patterns

See the [API Reference](../api/core.md) for complete API documentation.
