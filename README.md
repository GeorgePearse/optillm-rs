# optillm-rs

A Rust monorepo for implementations of optillm optimization techniques for LLMs. Provides multiple optimization strategies with a clear architecture for adding new implementations.

> **Note**: This is a port of the Python [OptimLLM](https://github.com/codelion/optillm) library designed to seamlessly integrate advanced LLM optimization strategies into the [code](https://github.com/just-every/code) project (Codex fork). This Rust implementation enables high-performance deployment of OptimLLM techniques within Rust-based systems while maintaining API compatibility with the original research implementations.

## Parity with upstream optillm

`crates/proxy` (`optillm-proxy`, also available as `optillm serve`) is a drop-in
Rust replacement for the upstream optillm server. The parity target is
**optillm v0.4.0, commit `e4ad19906578c9c472a3f8fd04d218db89a31c8c`**
(PyPI `optillm==0.4.0`).

```bash
cargo build --release -p optillm-proxy
OPENAI_API_KEY=sk-... ./target/release/optillm-proxy --approach moa --port 8000
# or: optillm serve --approach moa --port 8000
```

### What "perfectly match" means

1. **Proxy surface.** An OpenAI-compatible server (`/v1/chat/completions`, with
   streaming, plus `/v1/models` and `/health`) that selects approaches the way
   upstream does: a model-name slug prefix (`moa-gpt-4o`), `optillm_approach`
   in the request body / `extra_body`, an `<optillm_approach>` tag in the prompt,
   and the combinators `&` (pipeline) and `|` (parallel). It takes the same CLI
   flags and environment variables as upstream `server.py` (`--approach`, `--n`,
   `--base-url`, `--port`, `OPTILLM_API_KEY`, ...). Responses have the same
   shape, including `usage.completion_tokens` accounting, byte for byte after
   wall-clock ids and timestamps are normalised.
2. **Behavioural parity per approach and plugin.** A deterministic mock
   OpenAI backend (`parity/mock_backend.py`) records every request and returns
   scripted responses. Upstream optillm (`pip install optillm==0.4.0` in its own
   venv) and the Rust server run against the same mock with the same inputs. The
   harness (`parity/run_parity.py`) asserts that both make the same sequence of
   LLM calls (messages, system prompts, `n`, `temperature`, `max_tokens` and
   every other body field) and return the same final answer. It runs in CI
   (`.github/workflows/parity.yml`), with one fixture set per approach and plugin
   in `parity/fixtures/`.
3. **Local-inference approaches** (cot_decoding, entropy_decoding, thinkdeeper,
   deepconf, autothink) match upstream's local-inference behaviour where that is
   feasible. Otherwise they implement the same algorithm against a backend that
   exposes logprobs, and the gap is stated in the table below. Parity is never
   claimed for something the harness does not check.
4. **Plugins with external side effects** (web_search, readurls, executecode,
   mcp, deep_research) match the call structure, with network access and code
   execution gated behind the same flags upstream uses.

Prompts are copied verbatim from upstream (Apache-2.0, see `NOTICE`); multi-line
prompts are converted with `parity/tools/py2rs_prompt.py` so whitespace survives.
Stochastic approaches are compared under a fixed seed: the harness reseeds
upstream's `random` per request (`parity/upstream_hooks/sitecustomize.py`), and
the Rust server uses a bit-exact port of CPython's MT19937 (`OPTILLM_RANDOM_SEED`).

```bash
# run the harness locally
uv venv -p 3.12 ~/optillm-upstream/venv
VIRTUAL_ENV=~/optillm-upstream/venv uv pip install -r parity/requirements-upstream.txt
cargo build -p optillm-proxy
python parity/run_parity.py --upstream-python ~/optillm-upstream/venv/bin/python \
    --rust-bin target/debug/optillm-proxy
```

### Parity table

<!-- parity-table:start -->
| Kind | Name | Status | Fixtures | Known deviations / notes |
|---|---|---|---|---|
| Server | `proxy surface` | ✅ parity | 11 | /v1/chat/completions, /v1/models, /health, bearer-key check, Flask jsonify byte format, werkzeug 404/415 pages |
| Server | `approach selection` | ✅ parity | 15 | slug prefix, optillm_approach in body, <optillm_approach> tags, & pipeline, \| parallel, n repeats, untagging of User:/Assistant: output |
| Server | `none (passthrough)` | ✅ parity | 17 | openai-python model_dump shape, create() kwarg TypeError, 2 retries on 5xx/429, SSE re-chunking incl. tool calls and usage |
| Server | `batching` | ✅ parity | 5 | --batch-mode/--batch-size/--batch-wait-ms; per model:approach queues, sequential processing, wrapped error messages |
| Server | `conversation logging` | 🟡 implemented, not yet harness-verified | 0 | --log-conversations JSONL ported; not compared by the harness (timestamps, uuids) |
| Server | `provider selection` | 🟠 partial | 0 | OPENAI_API_KEY, CEREBRAS_API_KEY, AZURE_OPENAI_API_KEY supported. Gaps: OPTILLM_API_KEY local PyTorch inference engine and the LiteLLM fallback are not ported; Azure AD (DefaultAzureCredential) auth not supported; --launch-gui (Gradio) not supported |
| Approach | `bon` | ✅ parity | 7 | — |
| Approach | `moa` | ✅ parity | 5 | — |
| Approach | `rto` | ✅ parity | 4 | — |
| Approach | `self_consistency` | ✅ parity | 4 | difflib.SequenceMatcher ported incl. autojunk |
| Approach | `cot_reflection` | ✅ parity | 4 | — |
| Approach | `re2` | ✅ parity | 4 | — |
| Approach | `mcts` | ✅ parity | 6 | seeded: CPython MT19937 random.choice; numpy log/sqrt replaced by f64 (only affects exact UCB ties) |
| Approach | `pvg` | ✅ parity | 3 | upstream module-global token counter (never reset, grows across requests) replicated |
| Approach | `rstar` | ✅ parity | 4 | concurrent asyncio rollouts reproduced; harness serialises mock replies so seeded draws line up |
| Approach | `plansearch` | ✅ parity | 4 | — |
| Approach | `leap` | ✅ parity | 7 | Python json/iteration error semantics for malformed example JSON |
| Approach | `mars` | ⬜ not started | 0 | crates/mars is an independent native design, not a port |
| Approach | `cepo` | ✅ parity | 8 | nested thread pools + as_completed reproduced; process-global mutable config; --cepo_bestofn_rating_type / --cepo_rating_model accepted as str (upstream argparse rejects them, use --cepo_config_file); majority vote uses the MCQ path only (math_verify is unreachable with non-empty completions) |
| Approach | `z3` | ✅ parity | 5 | solver code runs in CPython+z3+sympy subprocess ($OPTILLM_Z3_PYTHON) with upstream exec globals; harness normalises the exec frame path in tracebacks |
| Local-inference approach | `cot_decoding` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `entropy_decoding` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `thinkdeeper` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `deepconf` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `autothink` | ⬜ not started | 0 | upstream: local inference only |
| Plugin | `coc` | ⬜ not started | 0 | — |
| Plugin | `compact` | ⬜ not started | 0 | — |
| Plugin | `deep_research` | ⬜ not started | 0 | — |
| Plugin | `deepthink` | ⬜ not started | 0 | — |
| Plugin | `executecode` | ⬜ not started | 0 | — |
| Plugin | `genselect` | ⬜ not started | 0 | — |
| Plugin | `json` | ⬜ not started | 0 | — |
| Plugin | `longcepo` | ⬜ not started | 0 | — |
| Plugin | `majority_voting` | ⬜ not started | 0 | — |
| Plugin | `mcp` | ⬜ not started | 0 | — |
| Plugin | `memory` | ⬜ not started | 0 | — |
| Plugin | `privacy` | ⬜ not started | 0 | — |
| Plugin | `proxy` | ⬜ not started | 0 | — |
| Plugin | `readurls` | ⬜ not started | 0 | — |
| Plugin | `router` | ⬜ not started | 0 | — |
| Plugin | `spl` | ⬜ not started | 0 | — |
| Plugin | `web_search` | ⬜ not started | 0 | — |

17/42 rows at harness-verified parity, 113 fixtures in total.
<!-- parity-table:end -->

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
│   └── AGENTS.md
├── .github/
│   └── workflows/
├── .gitignore
├── .memetics/
│   ├── adaptive-decoding-approximations.md
│   ├── mars-multi-agent-reasoning.md
│   ├── prompt-pipeline-strategies.md
│   ├── sampling-ensembles-moa-bon-self-consistency.md
│   └── tree-search-mcts-rstar.md
├── .optillm-parity-prompt.md
├── .pre-commit-config.yaml
├── Cargo.toml
├── INTEGRATION_GUIDE.md
├── NOTICE
├── README.md
├── STATUS.md
├── crates/
│   ├── cli/
│   ├── core/
│   ├── mars/
│   └── proxy/
├── docs/
│   ├── api/
│   ├── architecture/
│   ├── core/
│   ├── development/
│   ├── faq.md
│   ├── getting-started/
│   ├── index.md
│   ├── integration.md
│   ├── mars/
│   └── strategies/
├── examples/
│   ├── autothink_example.rs
│   ├── cot_decoding_example.rs
│   ├── deep_thinking_example.rs
│   ├── entropy_decoding_example.rs
│   └── r_star_example.rs
├── mkdocs.yml
├── modal_benchmark.py
├── parity/
│   ├── fixture_data/
│   ├── gen_fixtures.py
│   ├── mock_backend.py
│   ├── parity_table.py
│   ├── requirements-upstream.txt
│   ├── run_parity.py
│   ├── status.json
│   ├── tools/
│   └── upstream_hooks/
├── references/
│   └── optillm/
├── requirements-docs.txt
├── scratch_pads/
│   ├── CODING_LLM_BENCHMARKS.md
│   ├── COMPREHENSIVE_STRATEGY_BENCHMARK_RESULTS.md
│   ├── MODAL_BENCHMARK_SETUP.md
│   ├── TINYLLAMA_STRATEGY_TEST_RESULTS.md
│   ├── ULTRA_TINY_MODELS.md
│   └── UNIMPLEMENTED_TECHNIQUES.md
└── scripts/
    ├── check_syntax.py
    ├── detect_secrets.py
    └── update_readme_structure.py
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

optillm-rs is specifically designed to integrate with the [code](https://github.com/just-every/code) project, enabling it to leverage advanced OptimLLM reasoning strategies. This integration provides:

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

| Strategy | Description | Status |
|----------|-------------|--------|
| **MARS** (Multi-Agent Reasoning System) | Multi-agent exploration with cross-verification, aggregation, and iterative improvement for maximum quality. | ✅ Implemented |
| **MOA** (Mixture of Agents) | Three-phase approach: generate diverse completions, critique each, then synthesize optimal answer. | ✅ Implemented |
| **Self-Consistency** | Generate multiple diverse reasoning paths and use majority voting to reach consensus answers. | ✅ Implemented |
| **Best-of-N** | Generate N solutions with different parameters and select the highest quality based on scoring criteria. | ✅ Implemented |
| **RSA** (Reinforced Self-Aggregation) | Iteratively refine solutions by selecting diverse candidates and synthesizing improvements over multiple rounds. | ✅ Implemented |
| **MCTS** (Monte Carlo Tree Search) | Explore solution space systematically using UCB-based node selection and random simulation rollouts. | ✅ Implemented |
| **CoT Reflection** | Generate reasoning with chain-of-thought prompts and refine through self-reflection and error analysis. | ✅ Implemented |
| **RTO** (Round-Trip Optimization) | Improve answers through round-trip generation: forward pass then backward verification and refinement. | ✅ Implemented |
| **PVG** (Prover-Verifier Game) | Generate both helpful and adversarial solutions, then verify to identify robust answers. | ✅ Implemented |
| **LEAP** (Learning from Errors) | Use few-shot examples of corrected errors to adaptively improve solution quality over iterations. | ✅ Implemented |
| **PlanSearch** | Observation-guided problem solving combining planning phase with implementation and verification. | ✅ Implemented |
| **ReRead** | Simple but effective strategy of re-reading and refining answers for improved clarity and accuracy. | ✅ Implemented |
| **Diverse Sampling** | Explore solution space using temperature-varied sampling to balance exploration and exploitation. | ✅ Implemented |
| **AutoThink** | Query complexity classification with adaptive reasoning depth and temperature adjustment for optimal strategy selection. | ✅ Implemented |
| **Deep Thinking** | Inference-time scaling that allocates more computation and tokens to harder problems based on difficulty estimation. | ✅ Implemented |
| **Entropy Decoding** | Entropy-based sampling for controlled diversity, providing fine-grained control over answer quality versus novelty. | ✅ Implemented |
| **CoT Decoding** | Structured chain-of-thought decoding that guides models to follow step-by-step reasoning patterns for better quality. | ✅ Implemented |
| **R* Algorithm** | Enhanced Monte Carlo Tree Search with learned value estimates and sophisticated node selection for solution exploration. | ✅ Implemented |
| **CePO** (Cerebras Planning & Optimization) | Problem decomposition with state tracking and backtracking for systematic exploration of solution space. | ⏳ Planned |
| **Z3 Solver Integration** | Integration of Z3 SMT solver for constraint satisfaction and logical reasoning problems. | ⏳ Planned |
| **LongCePO** | Extended CePO with context windowing and recursive decomposition for handling problems with infinite context. | ⏳ Planned |

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
- **Code (Codex Fork)**: https://github.com/just-every/code - Integration target
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
