# Core Traits

The optillm-rs library is built around a few key traits that define the extension points and contracts for all implementations.

## Optimizer Trait

The `Optimizer` trait is the primary extension point for implementing new optimization techniques.

```rust
#[async_trait]
pub trait Optimizer {
    /// Run optimization on the given query
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution>;

    /// Human-readable name of the optimizer
    fn name(&self) -> &str;

    /// Description of what this optimizer does
    fn description(&self) -> &str;
}
```

### Implementing Optimizer

To create a new optimization technique:

```rust
pub struct MyOptimizer {
    // your configuration
}

#[async_trait]
impl Optimizer for MyOptimizer {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // Your implementation
        Ok(Solution::new(
            "my-optimizer".to_string(),
            reasoning,
            answer,
            0.5,
            token_count,
        ))
    }

    fn name(&self) -> &str {
        "My Optimizer"
    }

    fn description(&self) -> &str {
        "Description of your optimization technique"
    }
}
```

## ModelClient Trait

The `ModelClient` trait abstracts LLM provider interactions, enabling provider-agnostic code.

```rust
pub trait ModelClient: Send + Sync {
    /// Stream responses from the LLM
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> Pin<Box<dyn Stream<Item = Result<ResponseEvent>> + Send>>;
}
```

### Streaming Events

The LLM returns events as they're generated:

```rust
pub enum ResponseEvent {
    /// Text delta from streaming response
    OutputTextDelta { delta: String },

    /// Response completed with token usage
    Completed { token_usage: Option<TokenUsage> },
}
```

### Implementing ModelClient

To support a new LLM provider:

```rust
pub struct MyLLMClient {
    api_key: String,
    base_url: String,
}

impl ModelClient for MyLLMClient {
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> Pin<Box<dyn Stream<Item = Result<ResponseEvent>> + Send>> {
        Box::pin(async_stream::stream! {
            // Connect to your LLM API
            // Stream responses back
            // Yield ResponseEvent items
        })
    }
}
```

## Key Types

### Prompt

Represents a request to an LLM:

```rust
pub struct Prompt {
    pub input: Vec<ResponseItem>,
    pub base_instructions_override: Option<String>,
    pub log_tag: Option<String>,
}
```

### Solution

Result from an optimization technique:

```rust
pub struct Solution {
    pub agent_id: String,
    pub reasoning: String,
    pub answer: String,
    pub temperature: f32,
    pub token_count: usize,
}
```

### TokenUsage

Tracks token consumption:

```rust
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}
```

## Error Handling

All operations return a `Result<T>` type:

```rust
pub type Result<T> = std::result::Result<T, OptillmError>;

pub enum OptillmError {
    InvalidConfiguration(String),
    ClientError(String),
    ParsingError(String),
    // ... more variants
}
```

## Extension Points

1. **New Optimizers**: Implement `Optimizer` trait
2. **New Providers**: Implement `ModelClient` trait
3. **New Types**: Extend `Solution`, `Prompt`, etc.
4. **Event Handling**: Listen to `MarsEvent` stream
5. **Error Handling**: Use or extend error types

See [Creating New Optimizers](../development/creating-optimizers.md) for detailed implementation guides.
