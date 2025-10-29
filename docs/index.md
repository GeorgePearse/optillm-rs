# optillm-rs

A Rust monorepo for implementations of **OptimLLM** optimization techniques for LLMs. Provides multiple optimization strategies with a clear architecture for adding new implementations.

## 🎯 Overview

optillm-rs brings advanced LLM optimization techniques to Rust, enabling efficient inference through:

- **Multi-agent reasoning systems** (MARS) achieving 69% improvement on complex reasoning tasks
- **Diverse aggregation strategies** (MOA, tree search, best-of-N)
- **Strategy learning networks** for collective intelligence
- **Production-ready architecture** with streaming support and error handling

## 🚀 Quick Start

```bash
# Build all crates
cargo build --release

# Check without building
cargo check --all

# Build specific optimization strategy
cargo build --release -p optillm-mars
```

## 📊 Benchmark Results

**MARS (Multi-Agent Reasoning System)** achieves:

| Benchmark | Baseline | MARS | Improvement |
|-----------|----------|------|-------------|
| AIME 2025 | 43.3% | 73.3% | +69% |
| IMO 2025 | 16.7% | 33.3% | +100% |
| LiveCodeBench | 39.05% | 50.48% | +29% |

## 🏗️ Architecture

```
optillm-rs/
├── crates/
│   ├── core/          # Shared traits and interfaces
│   └── mars/          # MARS implementation
└── docs/              # This documentation
```

## 📚 Key Components

### optillm-core
Shared foundation providing:
- `ModelClient` trait for LLM communication
- `Optimizer` trait for implementations
- Unified types and error handling

### optillm-mars
Production MARS implementation with:
- Multi-agent exploration with diverse temperatures
- Cross-agent verification with consensus scoring
- RSA-inspired solution aggregation
- Strategy network for collective learning
- Real-time event streaming

## 🔧 What's Inside

- **Multi-Agent Systems**: Explore different solution paths in parallel
- **Verification & Aggregation**: Consensus-based solution refinement
- **Strategy Learning**: Extract and share successful reasoning patterns
- **Pluggable Architecture**: Easy to add new optimization strategies
- **Async-First Design**: Built for high-performance inference

## 📖 Documentation

- [Getting Started](getting-started/installation.md) - Installation and quick start
- [Architecture](architecture/overview.md) - System design and principles
- [MARS Guide](mars/overview.md) - Detailed MARS implementation
- [Development](development/contributing.md) - Contributing and extending

## 🎓 Example

```rust
use optillm_core::{ModelClient, Optimizer};
use optillm_mars::MarsCoordinator;

#[tokio::main]
async fn main() -> Result<()> {
    let config = MarsConfig::default();
    let coordinator = MarsCoordinator::new(config);

    let result = coordinator.optimize(
        "What is 2+2?",
        &your_model_client
    ).await?;

    println!("Answer: {}", result.answer);
    println!("Reasoning: {}", result.reasoning);

    Ok(())
}
```

## 🔗 References

- [OptimLLM GitHub](https://github.com/coohom/optillm) - Original Python implementation
- [MARS Research](https://arxiv.org/abs/2402.03666) - MARS paper and methodology
- [Rust Documentation](https://doc.rust-lang.org/) - Rust language reference

## 📝 License

MIT License - See LICENSE file for details

## 🤝 Contributing

Contributions welcome! See [Contributing Guide](development/contributing.md) for details.

---

**Last Updated**: October 2025
