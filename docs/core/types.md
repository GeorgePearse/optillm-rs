# Core Types

This document describes the fundamental types used throughout optillm-rs.

## Solution

The primary result type from optimization techniques.

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

### Creating a Solution

```rust
let solution = Solution::new(
    "agent-1".to_string(),
    "Because...".to_string(),
    "The answer is 42".to_string(),
    0.7,  // temperature
    1200, // token_count
);
```

### Solution Phase

```rust
pub enum GenerationPhase {
    Initial,
    Improved,
    Verified,
    Aggregated,
}
```

## Prompt

Represents a request to an LLM provider.

```rust
pub struct Prompt {
    pub input: Vec<ResponseItem>,
    pub base_instructions_override: Option<String>,
    pub log_tag: Option<String>,
}
```

### Building a Prompt

```rust
let mut prompt = Prompt::default();

// Add message
prompt.input = vec![ResponseItem::Message {
    id: None,
    role: "user".to_string(),
    content: vec![ContentItem::InputText {
        text: "Hello, world!".to_string(),
    }],
}];

// Override system prompt
prompt.base_instructions_override = Some(
    "You are a helpful assistant.".to_string()
);

// Add logging tag
prompt.set_log_tag("my-optimization");
```

## ResponseItem

Items in a prompt's conversation.

```rust
pub enum ResponseItem {
    Message {
        id: Option<String>,
        role: String,
        content: Vec<ContentItem>,
    }
}
```

Roles typically include:
- `"system"` - System instructions
- `"user"` - User message
- `"assistant"` - Assistant response

## ContentItem

Content within a message.

```rust
pub enum ContentItem {
    /// User input text
    InputText { text: String },

    /// Assistant response text
    Text { text: String },
}
```

## ResponseEvent

Events streamed from the LLM.

```rust
pub enum ResponseEvent {
    /// Text chunk from streaming response
    OutputTextDelta { delta: String },

    /// Response completed
    Completed { token_usage: Option<TokenUsage> },
}
```

### Handling Events

```rust
let mut stream = client.stream(&prompt);

while let Some(event) = stream.next().await {
    match event? {
        ResponseEvent::OutputTextDelta { delta } => {
            print!("{}", delta);
            total_text.push_str(&delta);
        }
        ResponseEvent::Completed { token_usage } => {
            if let Some(usage) = token_usage {
                println!("\nTokens: {} in, {} out",
                    usage.input_tokens,
                    usage.output_tokens);
            }
        }
    }
}
```

## TokenUsage

Tracks token consumption.

```rust
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl TokenUsage {
    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }
}
```

## OptillmError

Unified error type.

```rust
pub enum OptillmError {
    InvalidConfiguration(String),
    ClientError(String),
    ParsingError(String),
    // ... more variants
}
```

## Result Type Alias

```rust
pub type Result<T> = std::result::Result<T, OptillmError>;
```

Usage:

```rust
pub async fn my_function() -> Result<Solution> {
    // Returns Result<Solution, OptillmError>
}
```

## Type Relationships

```
Optimizer
├── accepts ModelClient
├── accepts Prompt
├── returns Solution
│   ├── contains reasoning: String
│   ├── contains answer: String
│   └── tracks phase: GenerationPhase
└── uses streaming
    └── receives ResponseEvent
        └── references TokenUsage

ModelClient
└── streams ResponseEvent items
    └── references TokenUsage
```

## Serialization

Most types implement `serde::Serialize` and `Deserialize`:

```rust
use serde::{Serialize, Deserialize};

let solution = Solution::new(/* ... */);
let json = serde_json::to_string(&solution)?;
let restored: Solution = serde_json::from_str(&json)?;
```

See [Error Handling](error-handling.md) for comprehensive error handling patterns.
