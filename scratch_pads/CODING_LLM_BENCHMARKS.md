# Local Coding LLM Benchmarks & Guide

This guide helps you find and test the smallest, fastest local coding LLMs for development with optillm-rs.

## TL;DR - Best Options

| Use Case | Model | Size | VRAM | Speed | Quality |
|----------|-------|------|------|-------|---------|
| **Best Overall** | DeepSeek Coder 6.7B | 6.7B | 4.5GB | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Fastest** | TinyLlama 1.1B | 1.1B | 1GB | ⭐⭐⭐⭐⭐ | ⭐⭐ |
| **Balanced** | Phi-3 Mini 3.8B | 3.8B | 2.5GB | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Production** | CodeLlama 7B | 7B | 5GB | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

---

## Model Comparison

### 🏆 DeepSeek Coder 6.7B (RECOMMENDED)

**Best balance of speed, quality, and size for coding**

```bash
ollama pull deepseek-coder:6.7b-base-q4_K_M
```

| Metric | Value |
|--------|-------|
| Parameters | 6.7B |
| Quantization | Q4_K_M (4-bit) |
| VRAM | ~4.5GB |
| Speed | ~25 tokens/sec |
| Training Focus | Coding/function generation |
| License | Proprietary |
| Quality | Excellent for code |

**Pros:**
- Purpose-built for coding tasks
- Excellent code quality
- Reasonable inference speed
- Good VRAM/quality tradeoff

**Cons:**
- Requires 4.5GB VRAM
- Proprietary (Deepseek)
- Slower than smaller models

**Best For:**
- Professional developers
- Production code generation
- Complex coding tasks
- Quality-first development

---

### ⚡ TinyLlama 1.1B (FASTEST)

**Smallest and fastest - runs everywhere**

```bash
ollama pull tinyllama
```

| Metric | Value |
|--------|-------|
| Parameters | 1.1B |
| Quantization | F16/Q4 |
| VRAM | ~1GB |
| Speed | ~60 tokens/sec |
| Training Focus | General instruction-following |
| License | MIT |
| Quality | Fair for simple code |

**Pros:**
- Only 1GB VRAM (runs on any CPU)
- Extremely fast (60 tok/sec)
- MIT licensed
- Great for iteration

**Cons:**
- Limited code quality
- General purpose (not code-specialized)
- May need optimization strategies

**Best For:**
- Resource-constrained systems
- Fast iteration/prototyping
- Brainstorming/planning
- Learning/experimentation

---

### 💎 Phi-3 Mini 3.8B (BALANCED)

**Microsoft's optimized small model**

```bash
ollama pull phi:mini
```

| Metric | Value |
|--------|-------|
| Parameters | 3.8B |
| Quantization | Q4 |
| VRAM | ~2.5GB |
| Speed | ~40 tokens/sec |
| Training Focus | Instructions/coding |
| License | MIT |
| Quality | Good for code |

**Pros:**
- Microsoft optimized (well-trained)
- MIT licensed
- Good speed/quality balance
- Lower VRAM than DeepSeek

**Cons:**
- General purpose (not code-specialized)
- Slower than TinyLlama
- Smaller than DeepSeek

**Best For:**
- Balanced development
- Mix of code/text tasks
- Resource-limited but quality-conscious
- MIT-licensed projects

---

### 📚 CodeLlama 7B (PRODUCTION)

**Meta's specialized coding model**

```bash
ollama pull codellama:7b-base-q4_K_M
```

| Metric | Value |
|--------|-------|
| Parameters | 7B |
| Quantization | Q4_K_M |
| VRAM | ~5GB |
| Speed | ~20 tokens/sec |
| Training Focus | Code generation |
| License | Llama 2 |
| Quality | Excellent for code |

**Pros:**
- Specialized for coding
- Excellent code quality
- Meta-backed (well-maintained)
- Strong for complex tasks

**Cons:**
- Requires 5GB VRAM
- Slowest of the options
- Slightly larger

**Best For:**
- Production systems
- High-quality code needed
- Complex algorithms
- Preference for quality over speed

---

## Running Benchmarks

### Quick Test (DeepSeek Coder)

```bash
# Terminal 1: Start Ollama
ollama serve

# Terminal 2: Pull the model
ollama pull deepseek-coder:6.7b-base-q4_K_M

# Terminal 3: Run the test
cargo run --example test_deepseek_coder
```

### Full Benchmark (All Models)

```bash
# Pull all models
ollama pull deepseek-coder:6.7b-base-q4_K_M
ollama pull phi:mini
ollama pull tinyllama
ollama pull codellama:7b-base-q4_K_M

# Run benchmark
cargo run --example test_coding_models
```

---

## Usage with Optimization Strategies

### For Small Models (1-4B)

Use lightweight strategies that don't require multiple expensive generations:

```rust
use optillm_mars::{
    ReReadAggregator, ReReadConfig,
    DiverseSamplingAggregator, DiverseSamplingConfig,
    providers::ollama::{OllamaClient, OllamaConfig},
};

// Strategy 1: ReRead (simple but effective)
let config = ReReadConfig::new();
let result = ReReadAggregator::run_reread(
    query,
    system_prompt,
    config,
    &client,
).await?;

// Strategy 2: Diverse Sampling (explore temperature range)
let config = DiverseSamplingConfig::new()
    .with_num_samples(3)  // Only 3 samples to save tokens
    .with_min_temperature(0.1)
    .with_max_temperature(0.7);
let result = DiverseSamplingAggregator::run_diverse_sampling(
    query,
    system_prompt,
    config,
    &client,
).await?;
```

### For Medium Models (6-7B)

Use moderate strategies that balance quality and cost:

```rust
use optillm_mars::{
    SelfConsistencyAggregator, SelfConsistencyConfig,
    BestOfNAggregator, BestOfNConfig,
    providers::ollama::{OllamaClient, OllamaConfig},
};

// Strategy 1: Self-Consistency (voting)
let config = SelfConsistencyConfig::new()
    .with_num_sampling_paths(5);
let result = SelfConsistencyAggregator::run_self_consistency(
    query,
    system_prompt,
    config,
    &client,
).await?;

// Strategy 2: Best-of-N
let config = BestOfNConfig::new()
    .with_num_samples(5);
let result = BestOfNAggregator::run_best_of_n(
    query,
    system_prompt,
    config,
    &client,
).await?;
```

### For Production (7B+)

Use advanced strategies for maximum quality:

```rust
use optillm_mars::{
    RSAAggregator, RSAConfig,
    PVGAggregator, PVGConfig,
    LEAPAggregator, LEAPConfig,
    providers::ollama::{OllamaClient, OllamaConfig},
};

// Strategy 1: RSA (Reinforced Self-Aggregation)
let config = RSAConfig::new()
    .with_aggregation_passes(3);
let result = RSAAggregator::run_rsa(
    query,
    system_prompt,
    config,
    &client,
).await?;

// Strategy 2: PVG (Prover-Verifier Game)
let config = PVGConfig::new();
let result = PVGAggregator::run_pvg(
    query,
    system_prompt,
    config,
    &client,
).await?;

// Strategy 3: LEAP (Learning from Errors)
let config = LEAPConfig::new()
    .with_num_learning_examples(3);
let result = LEAPAggregator::run_leap(
    query,
    system_prompt,
    config,
    &client,
).await?;
```

---

## Temperature Settings for Coding

```rust
// Very deterministic (for bug fixes, precision)
config.with_temperature(0.1)

// Balanced (general coding tasks)
config.with_temperature(0.3)

// Creative (brainstorming, exploring solutions)
config.with_temperature(0.7)

// Very creative (architecture, novel approaches)
config.with_temperature(1.2)
```

---

## Performance Expectations

### Single Generation Performance

```
Model               | Tokens/sec | Time for 500 tokens
DeepSeek 6.7B      | ~25       | ~20 seconds
CodeLlama 7B       | ~20       | ~25 seconds
Phi-3 Mini 3.8B    | ~40       | ~12 seconds
TinyLlama 1.1B     | ~60       | ~8 seconds
```

### With Optimization Strategies

```
Strategy           | Multiplier | Example (DeepSeek 6.7B)
Single             | 1x         | ~20 seconds for 500 tokens
Best-of-N (3)      | 3x         | ~60 seconds
Self-Consistency (5) | 5x        | ~100 seconds
RSA (3 passes)     | ~8x        | ~160 seconds
```

---

## System Requirements

### Minimum (TinyLlama)
- **CPU**: Any modern processor
- **RAM**: 4GB
- **Storage**: 2GB
- **GPU**: Optional (CPU runs fine)

### Recommended (Phi-3 Mini)
- **CPU**: 4+ cores
- **RAM**: 8GB
- **Storage**: 5GB
- **GPU**: Optional (4GB VRAM if available)

### Optimal (DeepSeek/CodeLlama)
- **CPU**: 8+ cores
- **RAM**: 16GB
- **Storage**: 10GB
- **GPU**: Recommended (NVIDIA 6GB+ or Apple Silicon)

---

## Installation & Setup

### 1. Install Ollama

**macOS:**
```bash
brew install ollama
```

**Linux:**
```bash
curl -fsSL https://ollama.ai/install.sh | sh
```

**Windows/Docker:**
Download from https://ollama.ai/download

### 2. Start Ollama Server

```bash
ollama serve
# Runs on http://localhost:11434
```

### 3. Pull Models

```bash
# Recommended
ollama pull deepseek-coder:6.7b-base-q4_K_M

# Or try others
ollama pull phi:mini
ollama pull tinyllama
ollama pull codellama:7b-base-q4_K_M
```

### 4. List Models

```bash
ollama list
```

---

## Troubleshooting

### "Failed to connect to Ollama"
```bash
# Ensure Ollama is running
ollama serve

# Check it's accessible
curl http://localhost:11434/api/tags
```

### "Model not found"
```bash
# Pull the model
ollama pull deepseek-coder:6.7b-base-q4_K_M

# Check installed models
ollama list
```

### Out of Memory
```bash
# Use a smaller model
ollama pull phi:mini  # 2.5GB instead of 4.5GB
ollama pull tinyllama # 1GB

# Or reduce num_predict
config.with_num_predict(512)  // instead of 1024
```

### Slow Performance
```bash
# Check CPU/GPU usage
# If using CPU, consider GPU:
#   - NVIDIA: Install CUDA
#   - Apple Silicon: Automatic
#   - AMD: ROCm support coming

# Or use smaller model
ollama pull tinyllama  // 60 tokens/sec
```

---

## Switching Between Models

Once you've chosen a model, swap easily in code:

```rust
// Development
let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "tinyllama".to_string(),  // Fast iteration
);

// Production
let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "deepseek-coder:6.7b-base-q4_K_M".to_string(),  // High quality
);
```

Or use environment variables:

```rust
let model = std::env::var("CODING_MODEL")
    .unwrap_or("tinyllama".to_string());
let config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    model,
);
```

---

## Comparison with Cloud APIs

| Aspect | Ollama Local | OpenAI API | Claude API |
|--------|--------------|-----------|-----------|
| **Cost** | Free* | $$ per token | $$ per token |
| **Privacy** | 100% local | Uploaded | Uploaded |
| **Speed** | Variable | Fast | Fast |
| **Quality** | Good | Excellent | Excellent |
| **Setup** | Easy | API key | API key |
| **Offline** | ✅ | ❌ | ❌ |
| **Latency** | 10-50ms | 100-500ms | 100-500ms |

*Compute cost (your hardware)

---

## Advanced Usage

### Custom Temperature Profiles

```rust
// Brainstorming (high temp, high creativity)
config
    .with_temperature(1.2)
    .with_top_p(0.95)
    .with_top_k(50)

// Precise code (low temp, deterministic)
config
    .with_temperature(0.1)
    .with_top_p(0.5)
    .with_top_k(20)

// Balanced (middle ground)
config
    .with_temperature(0.5)
    .with_top_p(0.9)
    .with_top_k(40)
```

### Combine with Optimization Strategies

```rust
// For highest quality code
let strategy_config = SelfConsistencyConfig::new()
    .with_num_sampling_paths(5);

let ollama_config = OllamaConfig::new(
    "http://localhost:11434".to_string(),
    "deepseek-coder:6.7b-base-q4_K_M".to_string(),
)
.with_temperature(0.3);  // Low for consistency

let client = OllamaClient::new(ollama_config)?;

let result = SelfConsistencyAggregator::run_self_consistency(
    query,
    system_prompt,
    strategy_config,
    &client,
).await?;
```

---

## Next Steps

1. **Try the benchmarks:**
   ```bash
   cargo run --example test_deepseek_coder
   cargo run --example test_coding_models
   ```

2. **Choose your model** based on requirements

3. **Integrate with optillm-rs** using provided examples

4. **Optimize with strategies** for better quality

5. **Switch to cloud APIs** if needed for production

---

## References

- **Ollama**: https://ollama.ai
- **DeepSeek Coder**: https://github.com/deepseek-ai/DeepSeek-Coder
- **CodeLlama**: https://github.com/meta-llama/codellama
- **Phi-3**: https://huggingface.co/microsoft/Phi-3-mini
- **TinyLlama**: https://huggingface.co/TinyLlama/TinyLlama-1.1B

