# Quick Start

Get your first optillm optimizer running in 5 minutes.

## 1. Create a New Project

```bash
cargo new my-optillm-app
cd my-optillm-app
```

## 2. Add Dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
optillm-core = { path = "../optillm-rs/crates/core" }
optillm-mars = { path = "../optillm-rs/crates/mars" }
```

## 3. Implement ModelClient

Create `src/model_client.rs`:

```rust
use optillm_core::{ModelClient, Prompt, ResponseEvent, OptillmError};
use std::pin::Pin;
use futures::Stream;

pub struct MyModelClient {
    api_key: String,
}

impl MyModelClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl ModelClient for MyModelClient {
    fn stream(&self, _prompt: &Prompt) -> Pin<Box<dyn Stream<Item = Result<ResponseEvent, OptillmError>> + Send>> {
        // Implement your LLM API call here
        // This is a stub - real implementation would call an LLM service

        Box::pin(async_stream::stream! {
            yield Ok(ResponseEvent::OutputTextDelta {
                delta: "Sample response".to_string(),
                index: 0,
            });
            yield Ok(ResponseEvent::Completed {
                token_usage: optillm_core::TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    total_tokens: 15,
                },
            });
        })
    }
}
```

## 4. Write Your First Program

Edit `src/main.rs`:

```rust
mod model_client;

use model_client::MyModelClient;
use optillm_mars::{MarsCoordinator, MarsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your model client
    let client = MyModelClient::new("your-api-key".to_string());

    // Configure MARS
    let config = MarsConfig::default();
    let coordinator = MarsCoordinator::new(config);

    // Run optimization
    let query = "What is 2 + 2?";
    let result = coordinator.optimize(query, &client).await?;

    // Print results
    println!("Query: {}", query);
    println!("Answer: {}", result.answer);
    println!("Reasoning: {}", result.reasoning);

    Ok(())
}
```

## 5. Run Your Program

```bash
cargo run
```

## Understanding the Output

The MARS coordinator returns:
- **`answer`** - The final optimized answer
- **`reasoning`** - The reasoning process used
- **`all_solutions`** - All solutions generated during exploration
- **`verifications`** - Verification results for each solution

## Using Different Strategies

### MOA (Mixture of Agents)

```rust
use optillm_mars::MoaAggregator;

// Generate diverse completions and synthesize
let (solution, metadata) = MoaAggregator::run_moa(
    "Your query",
    "System prompt",
    3,  // number of completions
    true,  // fallback enabled
    &client,
).await?;

println!("MOA Metadata: {:?}", metadata);
```

### MCTS (Monte Carlo Tree Search)

```rust
use optillm_mars::mcts::{MCTS, MCTSConfig};

let config = MCTSConfig::default();
let mut mcts = MCTS::new(config);

// Search for best dialogue path
let result = mcts.search(initial_state, &client).await?;
```

## Configuration

Customize MARS behavior:

```rust
use optillm_mars::MarsConfig;

let config = MarsConfig {
    num_agents: 5,
    temperatures: vec![0.3, 0.6, 0.9, 1.0, 1.2],
    max_iterations: 3,
    verification_threshold: 0.7,
    ..Default::default()
};

let coordinator = MarsCoordinator::new(config);
```

## Common Patterns

### Error Handling

```rust
match coordinator.optimize(query, &client).await {
    Ok(result) => println!("Success: {}", result.answer),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Streaming Results

MARS supports streaming events for real-time progress:

```rust
use optillm_mars::MarsEvent;

// Configure event handling
// This is done through the coordinator's event stream
```

### Batch Processing

```rust
let queries = vec![
    "Question 1?",
    "Question 2?",
    "Question 3?",
];

for query in queries {
    let result = coordinator.optimize(query, &client).await?;
    println!("Query: {} -> Answer: {}", query, result.answer);
}
```

## Next Steps

- [Examples](examples.md) - More detailed examples
- [MARS Guide](../mars/overview.md) - Deep dive into MARS
- [API Reference](../api/mars.md) - Full API documentation
- [Architecture](../architecture/overview.md) - Understand the design

## Troubleshooting

### Compilation Errors

```bash
# Update dependencies
cargo update

# Clean build
cargo clean
cargo build
```

### Runtime Errors

Check that your `ModelClient` implementation properly:
- Creates valid `Prompt` objects
- Streams `ResponseEvent` items correctly
- Handles errors appropriately

### Performance Tips

- Use `--release` builds for production
- Tune `num_agents` and `max_iterations` based on your needs
- Monitor token usage to optimize costs

## Getting Help

- Check the [FAQ](../faq.md)
- See [Examples](examples.md) for more patterns
- Review [API docs](../api/mars.md)
