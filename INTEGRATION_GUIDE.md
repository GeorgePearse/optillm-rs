# OptimLLM Integration Guide

Complete guide for integrating OptimLLM CLI into your coding CLI or LLM-powered system.

## Overview

OptimLLM provides two main integration paths:

1. **CLI Binary** - Use the `optillm` command-line tool directly
2. **Library Integration** - Use Rust libraries in your own code

## CLI Integration (Fastest)

### Installation

```bash
# Download from GitHub Releases
wget https://github.com/GeorgePearse/optillm-rs/releases/download/v0.1.0/optillm-linux-x86_64
chmod +x optillm-linux-x86_64

# Or build from source
cargo install --path optillm-rs/crates/cli

# Or use Cargo binstall (fastest)
cargo binstall optillm-cli
```

### Shell Integration

Add a wrapper function to your CLI:

```bash
#!/bin/bash
# In your CLI script

optimize_with_autothink() {
    local query="$1"
    local system="${2:-You are a helpful assistant.}"

    optillm autothink \
        --query "$query" \
        --system "$system" \
        -vv  # Verbose logging
}

optimize_with_deep_thinking() {
    local query="$1"
    local min_tokens="${2:-256}"
    local max_tokens="${3:-2048}"

    optillm deep-thinking \
        --query "$query" \
        --min-tokens "$min_tokens" \
        --max-tokens "$max_tokens"
}

# Usage in your CLI
case "$STRATEGY" in
    autothink)
        optimize_with_autothink "$QUERY" "$SYSTEM"
        ;;
    deep-thinking)
        optimize_with_deep_thinking "$QUERY" "$MIN" "$MAX"
        ;;
esac
```

### Python Integration

```python
#!/usr/bin/env python3
import subprocess
import sys
import json

class OptimLLMClient:
    """Client for OptimLLM CLI"""

    def __init__(self, optillm_path: str = "optillm"):
        self.optillm_path = optillm_path

    def run_strategy(self, strategy: str, query: str, **kwargs) -> dict:
        """Run an OptimLLM strategy"""
        cmd = [
            self.optillm_path,
            strategy,
            "--query", query,
        ]

        # Add additional arguments
        for key, value in kwargs.items():
            if value is not None:
                cmd.append(f"--{key.replace('_', '-')}")
                if isinstance(value, bool):
                    if value:
                        # Boolean flag
                        pass
                else:
                    cmd.append(str(value))

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60
        )

        if result.returncode != 0:
            raise RuntimeError(f"OptimLLM error: {result.stderr}")

        return {
            "status": "success",
            "output": result.stdout,
            "stderr": result.stderr
        }

    def list_strategies(self) -> dict:
        """List available strategies"""
        result = subprocess.run(
            [self.optillm_path, "strategies"],
            capture_output=True,
            text=True
        )
        return {"strategies": result.stdout}

# Usage
if __name__ == "__main__":
    client = OptimLLMClient()

    # Run AutoThink
    result = client.run_strategy(
        "autothink",
        "What is the capital of France?",
        system="You are helpful"
    )
    print(result["output"])

    # List strategies
    strategies = client.list_strategies()
    print(strategies)
```

### Go Integration

```go
package main

import (
	"fmt"
	"os/exec"
	"strings"
)

type OptimLLMClient struct {
	BinaryPath string
}

func NewOptimLLMClient(binaryPath string) *OptimLLMClient {
	return &OptimLLMClient{BinaryPath: binaryPath}
}

func (c *OptimLLMClient) RunStrategy(strategy, query string, args ...string) (string, error) {
	cmd := exec.Command(c.BinaryPath, strategy)
	cmd.Args = append(cmd.Args, "--query", query)
	cmd.Args = append(cmd.Args, args...)

	output, err := cmd.Output()
	if err != nil {
		return "", err
	}

	return string(output), nil
}

// Usage
func main() {
	client := NewOptimLLMClient("optillm")

	result, err := client.RunStrategy(
		"autothink",
		"What is 2+2?",
		"--system", "You are helpful",
	)
	if err != nil {
		fmt.Println("Error:", err)
		return
	}

	fmt.Println(result)
}
```

## Library Integration (Most Powerful)

### Direct Rust Integration

Add dependencies to your `Cargo.toml`:

```toml
[dependencies]
optillm-core = "0.1"
optillm-mars = "0.1"
tokio = { version = "1.40", features = ["full"] }
```

Use in your code:

```rust
use optillm_mars::strategies::{AutoThinkAggregator, AutoThinkConfig};
use optillm_core::ModelClient;

// Implement ModelClient for your LLM provider
struct MyModelClient {
    // Your LLM client fields
}

#[async_trait::async_trait]
impl ModelClient for MyModelClient {
    fn stream(&self, prompt: &optillm_core::Prompt)
        -> std::pin::Pin<Box<dyn futures::stream::Stream<Item = optillm_core::Result<optillm_core::ResponseEvent>> + Send>>
    {
        // Implement streaming response
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AutoThinkConfig::default();
    let client = MyModelClient { /* ... */ };

    let (solution, metadata) = AutoThinkAggregator::run_autothink(
        "What is the capital of France?",
        "You are a helpful assistant.",
        config,
        &client,
    ).await?;

    println!("Reasoning: {}", solution.reasoning);
    println!("Answer: {}", solution.answer);
    println!("Complexity: {}", metadata.complexity_score);

    Ok(())
}
```

### Implementing ModelClient

ModelClient is the core trait for LLM integration. Here's how to implement it:

```rust
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

pub struct OpenAIClient {
    api_key: String,
    model: String,
}

#[async_trait::async_trait]
impl ModelClient for OpenAIClient {
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> Pin<Box<dyn Stream<Item = optillm_core::Result<ResponseEvent>> + Send>> {
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let messages = prompt.messages.clone();

        Box::pin(async_stream::stream! {
            // Call your LLM API
            let response = reqwest::Client::new()
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&serde_json::json!({
                    "model": model,
                    "messages": messages,
                    "stream": true,
                }))
                .send()
                .await;

            // Stream tokens
            match response {
                Ok(resp) => {
                    let mut stream = resp.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        if let Ok(chunk) = chunk {
                            let text = String::from_utf8_lossy(&chunk).to_string();
                            yield Ok(ResponseEvent::OutputTextDelta { delta: text });
                        }
                    }

                    yield Ok(ResponseEvent::Completed {
                        token_usage: Some(optillm_core::TokenUsage {
                            input_tokens: 0,
                            output_tokens: 0,
                        }),
                    });
                }
                Err(e) => {
                    yield Err(optillm_core::Error::Custom(e.to_string()));
                }
            }
        })
    }
}
```

## Strategy Selection Guide

Choose the right strategy for your use case:

```
Query Type                    → Recommended Strategy
─────────────────────────────────────────────────────────
Simple factual questions      → AutoThink (complexity: simple)
Complex analysis problems     → AutoThink (complexity: complex)
Variable difficulty dataset   → Deep Thinking
Creative brainstorming        → Entropy Decoding
Math/logic proofs            → CoT Decoding
Optimization problems        → R* Algorithm
Multi-perspective analysis   → MARS
```

### AutoThink Decision Tree

```
Input Query
    │
    ├─ Analyze Complexity (5 factors)
    │   ├─ Length analysis
    │   ├─ Vocabulary complexity
    │   ├─ Reasoning keywords
    │   ├─ Domain indicators
    │   └─ Structural complexity
    │
    └─ Select Temperature
        ├─ Simple (< 0.25) → temp = 0.3
        ├─ Medium (0.25-0.40) → temp = 0.6
        └─ Complex (> 0.40) → temp = 1.0
```

## Configuration

### CLI Configuration File

Create `~/.optillm/config.json`:

```json
{
  "api_key": "${OPENAI_API_KEY}",
  "model": "gpt-4",
  "api_base": "https://api.openai.com/v1",
  "system_prompt": "You are a helpful, expert assistant",
  "timeout": 60
}
```

Use with:
```bash
export OPTILLM_CONFIG=~/.optillm/config.json
optillm autothink --query "Your query"
```

### Programmatic Configuration

```rust
use optillm_mars::strategies::{AutoThinkConfig, AutoThinkAggregator};

let config = AutoThinkConfig {
    simple_temperature: 0.3,
    medium_temperature: 0.6,
    complex_temperature: 1.0,
};

// Use with aggregator
let result = AutoThinkAggregator::run_autothink(
    query,
    system,
    config,
    &client,
).await?;
```

## Performance Optimization

### Caching Responses

```python
import hashlib
import json
from functools import lru_cache

class CachedOptimLLM:
    def __init__(self):
        self.cache = {}

    def run_with_cache(self, strategy: str, query: str, **kwargs):
        # Generate cache key
        cache_key = hashlib.md5(
            json.dumps({
                "strategy": strategy,
                "query": query,
                "kwargs": kwargs
            }).encode()
        ).hexdigest()

        if cache_key in self.cache:
            return self.cache[cache_key]

        # Run strategy
        result = self.run_strategy(strategy, query, **kwargs)
        self.cache[cache_key] = result

        return result
```

### Parallel Execution

```rust
use futures::future::join_all;

#[tokio::main]
async fn run_multiple_strategies(
    query: &str,
    client: &dyn ModelClient,
) -> Result<()> {
    let futures = vec![
        AutoThinkAggregator::run_autothink(query, "system", Default::default(), client),
        DeepThinkingAggregator::run_deep_thinking(query, "system", Default::default(), client),
        EntropyDecodingAggregator::run_entropy_decoding(query, "system", Default::default(), client),
    ];

    let results = join_all(futures).await;

    for result in results {
        if let Ok((solution, metadata)) = result {
            println!("Answer: {}", solution.answer);
        }
    }

    Ok(())
}
```

## Troubleshooting

### Binary Not Found
```bash
# Check PATH
echo $PATH

# Add to PATH
export PATH="/path/to/optillm:$PATH"

# Or use full path
/path/to/optillm strategies
```

### API Key Issues
```bash
export OPENAI_API_KEY="your-key"
export OPTILLM_API_KEY="your-key"
optillm -v autothink --query "test"
```

### Timeout Issues
```bash
# Increase timeout (in config.json)
{"timeout": 300}

# Or via CLI (for strategies with timeout support)
optillm --timeout 300 autothink --query "complex query"
```

## Examples

See `crates/cli/README.md` and `examples/` directory for:
- AutoThink examples
- Deep Thinking examples
- Entropy Decoding examples
- CoT Decoding examples
- R* Algorithm examples

## API Reference

### Binary

```
USAGE:
    optillm [OPTIONS] <COMMAND>

OPTIONS:
    -v, --verbose          Enable verbose logging
    --no-color             Disable colored output
    -c, --config <FILE>    Configuration file path
    -h, --help             Show help

COMMANDS:
    autothink              Run AutoThink strategy
    deep-thinking          Run Deep Thinking strategy
    entropy-decoding       Run Entropy Decoding strategy
    cot-decoding           Run CoT Decoding strategy
    r-star                 Run R* Algorithm
    mars                   Run MARS strategy
    strategies             List available strategies
```

### Library

See documentation at:
- [optillm-core](crates/core/)
- [optillm-mars](crates/mars/)

## Next Steps

1. **Choose integration method** - CLI vs library
2. **Install OptimLLM** - Download binary or build from source
3. **Configure LLM provider** - Set up API keys and config
4. **Test integration** - Run sample queries
5. **Deploy** - Integrate into your production system

## Support

- GitHub Issues: https://github.com/GeorgePearse/optillm-rs/issues
- Documentation: https://georgepearse.github.io/optillm-rs/
- Examples: https://github.com/GeorgePearse/optillm-rs/tree/main/examples

## License

MIT
