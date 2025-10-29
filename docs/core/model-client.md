# ModelClient Trait

The `ModelClient` trait provides an abstraction for communicating with Language Models, enabling provider-agnostic code.

## Definition

```rust
pub trait ModelClient: Send + Sync {
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> Pin<Box<dyn Stream<Item = Result<ResponseEvent>> + Send>>;
}
```

## Streaming Interface

The `ModelClient` returns a streaming interface to handle large responses efficiently:

```rust
// Get a stream
let stream = client.stream(&prompt);

// Consume events
while let Some(event) = stream.next().await {
    match event? {
        ResponseEvent::OutputTextDelta { delta } => {
            println!("Received: {}", delta);
        }
        ResponseEvent::Completed { token_usage } => {
            if let Some(usage) = token_usage {
                println!("Tokens: input={}, output={}",
                    usage.input_tokens,
                    usage.output_tokens);
            }
            break;
        }
    }
}
```

## Implementing ModelClient

To implement a new LLM provider:

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
            // 1. Convert Prompt to provider-specific format
            let request = self.build_request(prompt)?;

            // 2. Send request and get response stream
            let mut response = self.http_client
                .post(&self.base_url)
                .json(&request)
                .send()
                .await?;

            // 3. Stream and parse responses
            while let Some(chunk) = response.chunk().await? {
                let text = String::from_utf8(chunk.to_vec())?;

                // 4. Parse provider-specific format
                if let Ok(event) = self.parse_event(&text) {
                    // 5. Emit ResponseEvent
                    yield Ok(ResponseEvent::OutputTextDelta {
                        delta: event.content,
                    });
                }
            }

            // 6. Emit completion event
            yield Ok(ResponseEvent::Completed {
                token_usage: Some(TokenUsage {
                    input_tokens: total_input,
                    output_tokens: total_output,
                }),
            });
        })
    }
}
```

## Request/Response Flow

1. **Prompt Creation**: User creates `Prompt` with messages
2. **Stream Request**: Pass prompt to `client.stream()`
3. **Response Events**: Receive events as they stream
4. **Parse Events**: Extract text deltas and metadata
5. **Emit Events**: Yield `ResponseEvent` items
6. **Completion**: Final event signals completion

## Prompt Structure

```rust
pub struct Prompt {
    /// Input messages
    pub input: Vec<ResponseItem>,

    /// Override system instructions
    pub base_instructions_override: Option<String>,

    /// Log tag for debugging
    pub log_tag: Option<String>,
}

pub enum ResponseItem {
    Message {
        id: Option<String>,
        role: String,
        content: Vec<ContentItem>,
    }
}

pub enum ContentItem {
    InputText { text: String },
    Text { text: String },
}
```

## Response Events

```rust
pub enum ResponseEvent {
    /// Text delta (part of response)
    OutputTextDelta { delta: String },

    /// Response completed
    Completed { token_usage: Option<TokenUsage> },
}

pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}
```

## Error Handling

```rust
pub type Result<T> = std::result::Result<T, OptillmError>;

pub enum OptillmError {
    ClientError(String),      // Connection/network issues
    ParsingError(String),     // Response parsing failed
    InvalidConfiguration(String), // Bad config
    // ... more variants
}
```

## Built-in Implementations

- **`OllamaClient`**: For local Ollama models
- **Custom implementations**: For any LLM API

See [Strategies](../strategies/provider-routing.md) for multi-provider support.
