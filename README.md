# optillm-rs

A Rust monorepo for implementations of optillm optimization techniques for LLMs. Provides multiple optimization strategies with a clear architecture for adding new implementations.

## Quick Start

```bash
# Build all crates
cargo build --release

# Run checks
cargo check --all

# Build specific implementation
cargo build --release -p optillm-mars

# Build core only
cargo build -p optillm-core
```

## Project Structure

```
optillm-rs/
├── Cargo.toml                 # Workspace root
├── README.md                  # This file
├── .gitignore
└── crates/
    ├── core/                  # Shared traits, types, interfaces
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs         # Core module exports
    │   │   ├── client.rs      # ModelClient trait
    │   │   ├── optimizer.rs   # Optimizer trait
    │   │   ├── solution.rs    # Solution types
    │   │   └── error.rs       # Shared error types
    │   └── README.md
    │
    ├── mars/                  # MARS implementation
    │   ├── Cargo.toml
    │   ├── README.md          # MARS-specific docs
    │   └── src/
    │       ├── lib.rs         # Module exports
    │       ├── coordinator.rs # 5-phase orchestrator
    │       ├── agent.rs       # Multi-agent system
    │       ├── verifier.rs    # Solution verification
    │       ├── aggregator.rs  # Solution aggregation
    │       ├── strategy.rs    # Strategy network
    │       ├── workspace.rs   # Solution storage
    │       ├── types.rs       # MARS-specific types
    │       ├── config.rs      # Configuration
    │       ├── prompts.rs     # Prompt templates
    │       ├── error.rs       # MARS errors
    │       └── core_compat.rs # Backward compat
    │
    ├── [future implementations]
    │   ├── beam-search/
    │   ├── monte-carlo-tree-search/
    │   ├── best-of-n/
    │   └── dspy/
    │
    └── README.md (this file)
```

## Crates

### optillm-core

Shared core library providing interfaces and types for all optillm implementations:

- **`ModelClient` trait**: Abstract interface for LLM communication with streaming support
- **`Optimizer` trait**: Interface all implementations must implement
- **`Prompt` / `ResponseEvent` types**: Unified request/response representation
- **`Solution` struct**: Result containing reasoning and answer
- **Error types**: Shared error handling across implementations

### optillm-mars

Production-ready MARS (Multi-Agent Reasoning System) implementation achieving **69% improvement on AIME 2025** benchmarks.

**Key Features:**
- Multi-agent exploration with diverse temperatures [0.3, 0.6, 1.0]
- Cross-agent verification with consensus scoring
- RSA-inspired solution aggregation for refinement
- Iterative improvement with feedback loops
- Strategy network for collective learning
- Real-time event streaming

**Benchmark Results:**
- AIME 2025: 43.3% → 73.3% (+69% relative improvement)
- IMO 2025: 16.7% → 33.3% (+100% relative improvement)
- LiveCodeBench: 39.05% → 50.48% (+29% relative improvement)

See [crates/mars/README.md](crates/mars/README.md) for detailed documentation.

## Creating New Implementations

To add a new optimization technique:

1. **Create a new crate in `crates/`**:
   ```bash
   cargo new crates/my-optimizer
   ```

2. **Implement the `Optimizer` trait** from `optillm-core`:
   ```rust
   use optillm_core::{Optimizer, Solution, ModelClient, Result};
   use async_trait::async_trait;

   pub struct MyOptimizer {
       // your config
   }

   #[async_trait]
   impl Optimizer for MyOptimizer {
       async fn optimize(
           &self,
           query: &str,
           client: &dyn ModelClient,
       ) -> Result<Solution> {
           // your implementation
       }

       fn name(&self) -> &str { "my-optimizer" }
       fn description(&self) -> &str { "..." }
   }
   ```

3. **Add to workspace** in root `Cargo.toml`:
   ```toml
   members = [
       "crates/core",
       "crates/mars",
       "crates/my-optimizer",  # Add this
   ]
   ```

4. **Make it depend on optillm-core** in `Cargo.toml`:
   ```toml
   [dependencies]
   optillm-core = { path = "../core" }
   ```

## Workspace Dependencies

All crates share workspace dependencies for consistency. Core dependencies include:

- **tokio**: Async runtime
- **serde/serde_json**: Serialization
- **async-trait**: Async trait support
- **uuid**: ID generation
- **chrono**: Timestamps
- **thiserror**: Error handling

See root `Cargo.toml` for full dependency list.

## Building and Testing

```bash
# Build all
cargo build --release

# Check all (faster than build)
cargo check --all

# Build specific crate
cargo build -p optillm-mars

# Check specific crate
cargo check -p optillm-core

# Build with all features
cargo build --all-features
```

## Architecture Principles

1. **Trait-Based Design**: All implementations implement common traits from `optillm-core`
2. **Separation of Concerns**: Core types separate from implementation details
3. **Async-First**: All I/O operations are async
4. **Type Safety**: Strong typing throughout
5. **Extensibility**: Easy to add new optimization techniques

## License

MIT

## 📚 Documentation

Complete documentation is available via MkDocs. Build and serve locally:

```bash
# Install dependencies
pip install -r requirements-docs.txt

# Serve documentation locally
mkdocs serve

# Build static site
mkdocs build
```

View online: [Documentation](docs/index.md)

## References

- **OptimLLM**: https://github.com/coohom/optillm
- **MARS Paper**: MARS implementation details in crates/mars/README.md
- **Benchmarks**: AIME 2025, IMO 2025, LiveCodeBench
