# Design Patterns

## Overview

The optillm-rs library follows several key design patterns to ensure maintainability, extensibility, and reliability.

## Trait-Based Design

All implementations follow a trait-based design pattern using the `Optimizer` and `ModelClient` traits from `optillm-core`. This allows:

- **Extensibility**: New optimization techniques can be implemented by implementing the `Optimizer` trait
- **Provider Agnostic**: Different LLM providers can be supported through the `ModelClient` trait
- **Composability**: Strategies can be combined and layered
- **Testability**: Easy to mock and test with trait implementations

## Builder Pattern

Configuration objects use the builder pattern for convenient setup:

```rust
let config = OllamaConfig::new("http://localhost:11434".to_string(), "llama2".to_string())
    .with_temperature(0.7)
    .with_num_predict(4096)
    .with_top_p(0.9);
```

## Strategy Pattern

Different optimization strategies implement the same `Optimizer` interface, allowing runtime selection:

```rust
let strategy: Box<dyn Optimizer> = match selected_strategy {
    "mars" => Box::new(MarsCoordinator::new(config)),
    "moa" => Box::new(MoaAggregator::new(config)),
    _ => Box::new(BestOfNAggregator::new(config)),
};

let result = strategy.optimize(query, client).await?;
```

## Async-First Architecture

All I/O operations are async using Tokio:

- **Non-blocking**: Concurrent operations don't block threads
- **Efficient**: Better resource utilization with fewer threads
- **Scalable**: Can handle many concurrent requests
- **Composable**: Async functions compose well with `.await`

## Separation of Concerns

Modules are organized by responsibility:

- **core**: Shared traits and types
- **strategies**: Optimization algorithms
- **providers**: LLM provider integrations
- **core components** (agent, verifier, etc.): Specific functionality

## Error Handling

Comprehensive error types using `thiserror`:

```rust
pub type Result<T> = std::result::Result<T, MarsError>;

pub enum MarsError {
    AgentError(String),
    VerificationError(String),
    AggregationError(String),
    // ... more variants
}
```

## Streaming Pattern

Real-time progress tracking through event streaming:

```rust
pub enum MarsEvent {
    ExplorationStarted { num_agents: usize },
    SolutionGenerated { solution_id: String, agent_id: String },
    SolutionVerified { solution_id: String, is_correct: bool, score: f32 },
    // ... more events
}
```

## Configuration Hierarchy

Configurations follow a hierarchical structure:

1. **Workspace dependencies** in root `Cargo.toml`
2. **Crate-specific configs** in each `Cargo.toml`
3. **Runtime configs** as Rust structs with validation

## Type Safety

Strong typing throughout to catch errors at compile time:

- **Generics**: Parameterized over LLM providers
- **Enums**: For controlled state transitions
- **Result types**: Explicit error handling
- **Newtype pattern**: For semantic distinction

See the [Core Traits](core-traits.md) documentation for detailed trait definitions.
