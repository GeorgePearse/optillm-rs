# optillm-rs

A Rust monorepo for implementations of optillm optimization techniques for LLMs. Provides multiple optimization strategies with a clear architecture for adding new implementations.

> **Note**: This is a port of the Python [OptimLLM](https://github.com/codelion/optillm) library designed to seamlessly integrate advanced LLM optimization strategies into the [code](https://github.com/codelion/code) project (Codex fork). This Rust implementation enables high-performance deployment of OptimLLM techniques within Rust-based systems while maintaining API compatibility with the original research implementations.

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
├── .claude/
│   ├── AGENTS.md
│   ├── CLAUDE.md
│   └── settings.local.json
├── .github/
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── README.md
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── client.rs
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       ├── optimizer.rs
│   │       └── solution.rs
│   │
│   └── mars/
│       ├── Cargo.toml
│       ├── README.md
│       └── src/
│           ├── config.rs
│           ├── core/
│           ├── core_compat.rs
│           ├── error.rs
│           ├── lib.rs
│           ├── providers/
│           ├── strategies/
│           └── types.rs
├── docs/
├── examples/
├── mkdocs.yml
├── scratch_pads/
│   ├── CODING_LLM_BENCHMARKS.md
│   ├── COMPREHENSIVE_STRATEGY_BENCHMARK_RESULTS.md
│   ├── MODAL_BENCHMARK_SETUP.md
│   ├── TINYLLAMA_STRATEGY_TEST_RESULTS.md
│   └── ULTRA_TINY_MODELS.md
└── modal_benchmark.py
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

## Integration with Code (Codex Fork)

optillm-rs is specifically designed to integrate with the [code](https://github.com/codelion/code) project, enabling it to leverage advanced OptimLLM reasoning strategies. This integration provides:

- **Drop-in Optimization**: Use any MARS agent or strategy directly in code's LLM pipeline
- **Transparent API Management**: All LLM calls route through a unified abstraction
- **Multi-Provider Support**: Easily switch between different LLM providers
- **Performance**: Rust's performance benefits available in code's optimization layer

### Integration Pattern

```rust
// In code's coordinator or agent system
use optillm_mars::MarsCoordinator;

let coordinator = MarsCoordinator::new(config);
let result = coordinator.optimize(query, &code_model_client).await?;
// Result integrates seamlessly with code's reasoning pipeline
```

## LiteLLM-rs Integration

optillm-rs integrates **litellm-rs** for unified LLM API management, providing:

### Multi-Provider Support
- **OpenAI** (GPT-4, GPT-4o, etc.)
- **Anthropic** (Claude 3, Claude 3.5, etc.)
- **Google** (Gemini models)
- **Groq** (Fast inference)
- **Local Models** (Ollama, vLLM, etc.)
- **Custom Endpoints**

### Benefits
- **Unified API**: Single interface for all providers
- **Automatic Routing**: Route requests based on cost, latency, or availability
- **Fallback Support**: Automatic failover to alternative providers
- **Cost Optimization**: Track and optimize token usage across providers
- **Zero Configuration**: Sensible defaults with easy customization

### Provider Configuration

```rust
use optillm_mars::provider_config::{ProviderSpec, ProviderRoutingConfig, RoutingStrategy};

// Configure multiple providers
let openai = ProviderSpec::new("openai", "gpt-4o")
    .with_api_key(env::var("OPENAI_API_KEY")?)
    .with_priority(1);

let anthropic = ProviderSpec::new("anthropic", "claude-3-5-sonnet")
    .with_api_key(env::var("ANTHROPIC_API_KEY")?)
    .with_priority(2);

let config = ProviderRoutingConfig::multi(openai, vec![anthropic])
    .with_strategy(RoutingStrategy::RoundRobin)
    .with_fallback(true)
    .with_max_retries(2);
```

### API Call Management Features

- **Automatic Retries**: Configurable retry logic with exponential backoff
- **Token Counting**: Accurate token usage tracking for cost management
- **Rate Limiting**: Respect provider-specific rate limits
- **Streaming Support**: Efficient streaming responses
- **Error Handling**: Comprehensive error handling for all providers

### Using with Code Integration

```rust
// In code's model initialization
use optillm_mars::model_router::ModelClientRouter;

// Wrap code's existing ModelClient with routing capabilities
let router = ModelClientRouter::new();
let coordinator = MarsCoordinator::new(config);

// Now MARS can route through multiple providers
let result = coordinator.optimize(query, &router).await?;
```

## Strategy Implementation for Code

The following strategies are available for integration into code:

### MARS (Multi-Agent Reasoning System)
- Full 5-phase optimization pipeline
- Multi-agent exploration with diverse temperatures
- Cross-agent verification and consensus scoring
- RSA-inspired solution aggregation
- Iterative improvement with feedback loops

### MOA (Mixture of Agents)
- Phase 1: Generate diverse completions
- Phase 2: Critique each completion
- Phase 3: Synthesize optimal answer
- Useful for creative and complex reasoning tasks

### MCTS (Monte Carlo Tree Search)
- UCB-based node selection
- Dialogue state exploration
- Tree-based reasoning discovery
- Low-cost reasoning exploration

### Custom Strategies
Add new strategies by implementing the `Optimizer` trait—perfect for domain-specific optimizations for code's specialized tasks.

## Usage in Code Projects

To use optillm-rs strategies in a code-based system:

1. **Add Dependency**: Include optillm-mars in your Cargo.toml
2. **Configure Providers**: Set up litellm-rs for your LLM endpoints
3. **Instantiate Coordinator**: Create a MARS or custom coordinator
4. **Optimize Queries**: Pass queries through the optimization pipeline
5. **Integrate Results**: Use optimized answers in code's reasoning flow

See [Integration Guide](docs/integration.md) for detailed instructions.

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

### Core Projects
- **OptimLLM** (Python): https://github.com/codelion/optillm - Original research implementation
- **Code (Codex Fork)**: https://github.com/codelion/code - Integration target
- **LiteLLM**: https://litellm.ai/ - Multi-provider LLM API management
- **LiteLLM-rs**: Rust bindings for unified LLM provider support

### Research & Benchmarks
- **MARS Paper**: Multi-Agent Reasoning System implementation and evaluation
- **AIME 2025**: 43.3% → 73.3% (+69% relative improvement)
- **IMO 2025**: 16.7% → 33.3% (+100% relative improvement)
- **LiveCodeBench**: 39.05% → 50.48% (+29% relative improvement)

### Documentation
- Full MARS implementation details: [crates/mars/README.md](crates/mars/README.md)
- Integration guide: [docs/integration.md](docs/integration.md)
- Strategy implementations: [docs/strategies/](docs/strategies/)
