# Ollama Local Model Integration

Ollama support enables optillm-rs to run optimization strategies with local language models, providing a completely offline-first inference experience with no API costs.

## What is Ollama?

Ollama is a simple, user-friendly tool for running open-source LLMs locally. It provides:
- **Zero infrastructure**: Lightweight daemon (~100MB footprint)
- **Huge model library**: 100+ pre-built models (Llama 2, Mistral, Neural Chat, etc.)
- **CPU/GPU support**: Works on CPU, Metal (Apple Silicon), and NVIDIA/AMD GPUs
- **OpenAI-compatible API**: Standard HTTP API for integration
- **One-command setup**: `ollama run llama2`

## Installation

### Quick Start

1. **Install Ollama**
   ```bash
   # macOS
   brew install ollama

   # Linux
   curl -fsSL https://ollama.ai/install.sh | sh

   # Windows/Docker
   # Download from https://ollama.ai/download
   ```

2. **Start Ollama server**
   ```bash
   ollama serve
   # Runs on http://localhost:11434
   ```

3. **Download a model**
   ```bash
   ollama pull llama2        # 3.8B, fastest
   ollama pull mistral       # 7B, better quality
   ollama pull neural-chat   # 7B, optimized for chat
   ```

## Configuration

### Basic Setup

```rust
use optillm_mars::providers::ollama::{OllamaClient, OllamaConfig};
use optillm_core::ModelClient;

// Create configuration
let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "mistral".to_string(),
);

// Create client
let client = OllamaClient::new(config)?;
```

### Configuration Options

| Option | Default | Range | Description |
|--------|---------|-------|-------------|
| `base_url` | - | - | Ollama server URL (e.g., `http://localhost:11434`) |
| `model` | - | - | Model name (e.g., `llama2`, `mistral`, `neural-chat`) |
| `temperature` | 0.7 | 0.0-2.0 | Creativity level (0=deterministic, 2=creative) |
| `num_predict` | 4096 | 1+ | Maximum tokens to generate |
| `top_p` | 0.9 | 0.0-1.0 | Nucleus sampling parameter |
| `top_k` | 40 | 1+ | Top-k sampling parameter |

### Builder Pattern

```rust
let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "mistral".to_string(),
)
.with_temperature(0.5)
.with_num_predict(2048)
.with_top_p(0.95)
.with_top_k(50);

let client = OllamaClient::new(config)?;
```

## Usage with Optimization Strategies

All MARS optimization strategies work seamlessly with Ollama:

### Self-Consistency (Voting)

```rust
use optillm_mars::{
    SelfConsistencyAggregator, SelfConsistencyConfig,
    providers::ollama::{OllamaClient, OllamaConfig},
};

let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "mistral".to_string(),
);
let client = OllamaClient::new(config)?;

let strategy_config = SelfConsistencyConfig::new().with_num_sampling_paths(5);

let result = SelfConsistencyAggregator::run_self_consistency(
    "What is 2 + 2?",
    "You are a helpful math assistant.",
    strategy_config,
    &client,
).await?;

println!("Final answer: {}", result.final_answer);
println!("Agreement: {}%", result.metadata.agreement_percentage);
```

### Best-of-N Selection

```rust
use optillm_mars::{
    BestOfNAggregator, BestOfNConfig,
    providers::ollama::{OllamaClient, OllamaConfig},
};

let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "llama2".to_string(),
).with_temperature(0.8);

let client = OllamaClient::new(config)?;

let strategy_config = BestOfNConfig::new().with_num_samples(5);

let result = BestOfNAggregator::run_best_of_n(
    "Write a haiku about nature",
    "You are a creative poet.",
    strategy_config,
    &client,
).await?;

println!("Best answer:\n{}", result.best_answer);
```

### Diverse Sampling (Temperature-Based Exploration)

```rust
use optillm_mars::{
    DiverseSamplingAggregator, DiverseSamplingConfig,
    providers::ollama::{OllamaClient, OllamaConfig},
};

let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "neural-chat".to_string(),
);
let client = OllamaClient::new(config)?;

let strategy_config = DiverseSamplingConfig::new()
    .with_num_samples(5)
    .with_min_temperature(0.1)
    .with_max_temperature(1.5);

let result = DiverseSamplingAggregator::run_diverse_sampling(
    "Explain quantum computing",
    "You are a physics teacher.",
    strategy_config,
    &client,
).await?;

println!("Deterministic (0.1°): {}", result.metadata.samples[0].answer);
println!("Creative (1.5°): {}", result.metadata.samples[4].answer);
```

## Performance Characteristics

### Model Selection Guide

| Model | Size | Speed | Quality | VRAM | Best For |
|-------|------|-------|---------|------|----------|
| **Llama 2** | 3.8B | Fastest | Good | 2GB | Dev/testing |
| **Mistral** | 7B | Fast | Better | 4GB | Balanced |
| **Neural Chat** | 7B | Fast | Excellent | 4GB | Chat/tasks |
| **Llama 2 13B** | 13B | Moderate | Excellent | 8GB | Production |
| **Mixtral** | 46B | Slow | Best | 16GB+ | High-quality |

### Typical Latency (on 8GB GPU)

```
Llama 2 (3.8B):  50-100 tokens/sec
Mistral (7B):    30-50 tokens/sec
Neural Chat:     30-50 tokens/sec
Llama 2 (13B):   15-30 tokens/sec
```

### Token Counting

Ollama returns accurate token counts:
```rust
let result = SelfConsistencyAggregator::run_self_consistency(
    query,
    system_prompt,
    config,
    &client,
).await?;

println!("Total tokens used: {}", result.metadata.total_tokens);
```

## Advanced Configuration

### Remote Ollama Server

```rust
// Connect to remote Ollama instance
let config = OllamaConfig::new(
    "http://ollama-server.example.com:11434".to_string(),
    "mistral".to_string(),
);
let client = OllamaClient::new(config)?;
```

### Temperature-Based Customization

```rust
// Deterministic (facts/code)
let deterministic = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "mistral".to_string(),
).with_temperature(0.1);

// Balanced
let balanced = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "mistral".to_string(),
).with_temperature(0.7);

// Creative (writing/brainstorming)
let creative = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "mistral".to_string(),
).with_temperature(1.5);
```

### Optimization Strategies for Local Models

**For CPU-constrained systems:**
- Use smaller models (Llama 2 3.8B)
- Reduce `num_predict` to 1024
- Use `Best-of-N` (fewer total generations)

**For GPU-enabled systems:**
- Use larger models (13B or larger)
- Increase `num_predict` to 4096
- Use `Self-Consistency` with more paths

**For latency-sensitive applications:**
- Use smaller models
- Reduce temperature (more deterministic)
- Limit strategies to 3-5 iterations

## Streaming Response Handling

The Ollama client automatically handles streaming responses, yielding text deltas in real-time:

```rust
use futures::StreamExt;
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};

let client = OllamaClient::new(config)?;

let mut prompt = Prompt::new();
prompt.input = vec![
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "Hello!".to_string(),
        }],
    },
];

let mut stream = client.stream(&prompt);

while let Some(event) = stream.next().await {
    match event? {
        ResponseEvent::OutputTextDelta { delta } => {
            print!("{}", delta);
        }
        ResponseEvent::Completed { token_usage } => {
            println!("\nCompleted!");
            if let Some(usage) = token_usage {
                println!("Tokens: {}", usage.total_tokens());
            }
        }
    }
}
```

## Troubleshooting

### "Failed to connect to Ollama"
- Ensure Ollama server is running: `ollama serve`
- Check URL is correct: `http://localhost:11434`
- For remote: ensure firewall allows access

### "Model not found"
- Pull the model: `ollama pull mistral`
- List available: `ollama list`
- Check model name exactly matches config

### "Out of memory"
- Use smaller model: `ollama pull llama2` (3.8B)
- Reduce `num_predict` in config
- Close other applications

### Slow inference
- GPU not being used: Check Ollama server logs
- Model too large: Try smaller model
- Network latency: For remote, check bandwidth

## Comparison: Ollama vs Remote APIs

| Aspect | Ollama | OpenAI/Claude API |
|--------|--------|-------------------|
| **Cost** | Free (compute only) | $$ per token |
| **Privacy** | 100% local, no data upload | Data sent to API |
| **Speed** | Variable (depends on hardware) | Fast (optimized servers) |
| **Model choice** | Many open models | Limited to provider models |
| **Quality** | Good (Mistral/Llama 13B) | Excellent (latest models) |
| **Setup** | Simple (one command) | API key required |
| **Offline** | ✅ Complete offline | ❌ Internet required |

## Best Practices

1. **Start with Ollama for development**
   - Test optimization strategies locally
   - Understand token costs
   - No API fees during development

2. **Use appropriate model size**
   - 3.8B (Llama 2): Development, testing
   - 7B (Mistral): Balanced tasks
   - 13B+: Production quality needed

3. **Monitor token usage**
   - Check `metadata.total_tokens` for strategies
   - Plan resource usage before scaling

4. **Cache model in memory**
   - After first load, subsequent requests are fast
   - Ollama keeps models loaded

5. **Combine with cloud APIs**
   - Use Ollama for development
   - Switch to OpenAI/Claude in production
   - Code is identical (same `ModelClient` trait)

## Future Enhancements

- [ ] GPU auto-detection and configuration
- [ ] Model downloading within client
- [ ] Built-in model recommendations
- [ ] Performance benchmarking utilities
- [ ] Multi-GPU support for larger models
- [ ] Model quantization helpers

## References

- **Ollama**: https://ollama.ai
- **Available Models**: https://ollama.ai/library
- **API Documentation**: https://github.com/ollama/ollama/blob/main/docs/api.md
- **GitHub**: https://github.com/ollama/ollama
