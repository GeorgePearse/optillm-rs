# API Reference: optillm-core

Complete API documentation for the optillm-core library.

## Traits

### Optimizer

Main trait for optimization implementations.

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

### ModelClient

Abstract interface for LLM providers.

```rust
pub trait ModelClient: Send + Sync {
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> Pin<Box<dyn Stream<Item = Result<ResponseEvent>> + Send>>;
}
```

## Types

### Solution

```rust
pub struct Solution {
    pub agent_id: String,
    pub reasoning: String,
    pub answer: String,
    pub temperature: f32,
    pub token_count: usize,
    pub is_verified: bool,
    pub verification_score: f32,
    pub phase: GenerationPhase,
}
```

### Prompt

```rust
pub struct Prompt {
    pub input: Vec<ResponseItem>,
    pub base_instructions_override: Option<String>,
    pub log_tag: Option<String>,
}
```

### ResponseEvent

```rust
pub enum ResponseEvent {
    OutputTextDelta { delta: String },
    Completed { token_usage: Option<TokenUsage> },
}
```

### TokenUsage

```rust
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}
```

## Error Types

### OptillmError

```rust
pub enum OptillmError {
    InvalidConfiguration(String),
    ClientError(String),
    ParsingError(String),
    Timeout(String),
}
```

### Result

```rust
pub type Result<T> = std::result::Result<T, OptillmError>;
```

## Complete API

For complete API documentation with all methods and fields, run:

```bash
cargo doc -p optillm-core --open
```

See [Core Library](../core/introduction.md) for usage guides.
