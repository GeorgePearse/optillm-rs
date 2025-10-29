# Modal Benchmark Setup for OptILLM-RS

## Overview

This document describes the benchmark infrastructure setup for testing all optimization strategies across different models using Modal.

## Benchmarks Implemented

### 1. **comprehensive_benchmark.rs** (Rust-based)
- Location: `crates/mars/examples/comprehensive_benchmark.rs`
- Tests all optimization strategies with local Ollama models
- Measures: latency, tokens, throughput, success rates
- Strategies tested:
  - ✅ Baseline (single pass)
  - ✅ ReRead (verification strategy)
  - ✅ Diverse Sampling (temperature variation)
  - ✅ Best-of-N (multiple attempts + selection)
- Models supported: TinyLlama 1.1B (and configurable)
- Run: `cargo run --example comprehensive_benchmark --release`

### 2. **modal_benchmark.py** (Python-based for Modal cloud)
- Location: `modal_benchmark.py` (root)
- Python/Modal app for cloud-based benchmarking
- Can run on Modal's serverless infrastructure
- Supports Ollama containers or API endpoints
- Generates comprehensive markdown reports
- Run: `modal run modal_benchmark.py`

## Architecture

### Local Testing (Rust Benchmark)
```
┌─────────────────────────────────────────────────────┐
│         Rust Benchmark Application                  │
├─────────────────────────────────────────────────────┤
│ Strategies:                                          │
│  • Baseline                                         │
│  • ReRead                                           │
│  • DiverseSampling                                  │
│  • BestOfN                                          │
├─────────────────────────────────────────────────────┤
│         OllamaClient (ModelClient trait)            │
├─────────────────────────────────────────────────────┤
│      Ollama (HTTP/REST API on localhost)           │
├─────────────────────────────────────────────────────┤
│         Local LLM Model (TinyLlama)                 │
└─────────────────────────────────────────────────────┘
```

### Modal Cloud Testing (Python Benchmark)
```
┌──────────────────────────────────────────────────┐
│         Modal App (Python)                        │
├──────────────────────────────────────────────────┤
│ Strategies:                                       │
│  • All optimization strategies                   │
│  • Multiple model sizes                          │
│  • Various test tasks                            │
├──────────────────────────────────────────────────┤
│    HTTP calls to local or remote models          │
├──────────────────────────────────────────────────┤
│  Cloud containers with Ollama or API endpoints   │
└──────────────────────────────────────────────────┘
```

## Prerequisites

### For Local Rust Benchmark
```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh

# Start Ollama service
systemctl start ollama
systemctl status ollama

# Pull model
ollama pull tinyllama

# Verify
ollama list
```

### For Modal Cloud Benchmark
```bash
# Install Modal CLI
pip install modal

# Authenticate
modal token new

# Verify
modal version
```

## Test Configuration

### Models
- **TinyLlama 1.1B** (local testing)
  - Size: 637MB
  - VRAM: ~1GB
  - Speed: 50-70 tokens/sec
  - Category: Ultra-lightweight

### Tasks
1. **prime_check**: Write Rust function to check if number is prime
2. **fibonacci**: Write Rust function for nth Fibonacci number

### Strategies
1. **Baseline**: Single pass inference (1x cost)
2. **ReRead**: Verification with re-reading (1.5x cost)
3. **Diverse Sampling**: Temperature variation sampling (3x cost)
4. **Best-of-N**: Multiple attempts with selection (3x cost)

## Running the Benchmarks

### Option 1: Local Rust Benchmark
```bash
# Build the benchmark
cargo build --example comprehensive_benchmark --release

# Run the benchmark
cargo run --example comprehensive_benchmark --release

# Expected output:
# ✓ Connected to tinyllama
# Strategy: BASELINE
#   [1/8] Task prime_check... ✓ 337 tokens, 12400ms, 27.1 tok/s
#   [2/8] Task fibonacci... ✓ 425 tokens, 15000ms, 28.3 tok/s
# ... (more results for other strategies)
# Strategy comparison table
# Results by model
# Recommendations
```

Estimated runtime: **20-30 minutes** (all strategies on all tasks)

### Option 2: Modal Cloud Benchmark
```bash
# Run on Modal infrastructure
modal run modal_benchmark.py

# Or run in daemon mode for long-running benchmarks
modal run -q modal_benchmark.py

# View results
cat MODAL_BENCHMARK_RESULTS.md
```

## Expected Results

### TinyLlama Performance Baseline
| Metric | Baseline | ReRead | Diverse Sampling | Best-of-N |
|--------|----------|--------|------------------|-----------|
| Tokens | 337-425 | 250-400 | 900-1100 | 1000-1200 |
| Latency | 12-15s | 17-20s | 35-40s | 40-50s |
| Throughput | 27-30 tok/s | 25-30 tok/s | 30-32 tok/s | 28-30 tok/s |
| Cost | 1x | 1.5x | 3x | 3x |
| Quality | ✅ Good | ⚠️ Variable | ❌ Mixed | ❌ Mixed |

### Benchmark Insights (from previous testing)
1. Small models (1.1B) work better with **baseline strategy**
2. Optimization strategies may degrade output quality on tiny models
3. Cost (tokens + time) increases 3x for strategies that don't improve quality
4. Recommendation: Use single-pass for models < 4B parameters

## Output Files

### Rust Benchmark Output
- Console: Detailed strategy results with metrics
- Format: Tables, comparisons, recommendations

### Modal Benchmark Output
- `MODAL_BENCHMARK_RESULTS.md`: Comprehensive markdown report including:
  - Summary statistics
  - Strategy comparison table
  - Results by model
  - Detailed results JSON
  - Analysis and recommendations

## Integration with optillm-rs

All benchmarks use the public APIs:
```rust
// Baseline
let mut stream = client.stream(&prompt);

// ReRead
let result = ReReadAggregator::run_reread(prompt, system, config, client).await?;

// Diverse Sampling
let result = DiverseSamplingAggregator::run_diverse_sampling(prompt, system, config, client).await?;

// Best-of-N
let (solution, metadata) = BestOfNAggregator::run_best_of_n(prompt, system, config, client).await?;
```

## Next Steps

1. **Run local benchmark** to establish baseline performance
2. **Compare with larger models** (7B, 13B) if needed
3. **Test additional strategies** as they're implemented
4. **Gather production metrics** from real workloads
5. **Deploy to Modal** for automated CI/CD benchmarking

## Troubleshooting

### Ollama Connection Issues
```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Check service status
systemctl status ollama

# Restart service
systemctl restart ollama
```

### Modal Authentication Issues
```bash
# Check authentication
modal token list

# Re-authenticate
modal token new

# Verify setup
modal version
modal whoami
```

### Slow Benchmarks
- Small models can be very slow on CPU
- Reduce `num_samples` in DiverseSampling config
- Reduce `num_predict` to limit max tokens
- Run individual strategies: update `STRATEGIES` constant

## References

- OptILLM Paper: https://arxiv.org/abs/2406.03588
- Modal Docs: https://modal.com/docs
- Ollama: https://ollama.ai/
- TinyLlama: https://github.com/jzhang38/TinyLlama
