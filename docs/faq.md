# Frequently Asked Questions

## General Questions

### What is optillm-rs?

optillm-rs is a Rust implementation of OptimLLM optimization techniques for large language models. It provides production-ready strategies for improving LLM reasoning quality through multi-agent exploration, verification, and aggregation.

### Why Rust?

Rust provides:
- **Performance**: Zero-cost abstractions and no garbage collection
- **Safety**: Memory safety without runtime overhead
- **Concurrency**: Excellent async/await support
- **Type Safety**: Compile-time guarantees reduce runtime errors
- **Productivity**: Cargo package manager and rich ecosystem

### How does it compare to the Python OptimLLM?

optillm-rs maintains functional parity with the Python version while offering:
- ✅ Better performance for production workloads
- ✅ Type safety and compile-time verification
- ✅ Native async/await for concurrent operations
- ✅ Lower memory footprint
- ❌ Smaller ecosystem (growing)

## Installation & Setup

### Can I use optillm-rs in my Python project?

Currently no, but you can:
1. Build optillm-rs as a service and call it via HTTP/gRPC
2. Use PyO3 bindings (not yet available)
3. Run as a separate Rust service

### What Rust version is required?

Minimum Rust 1.70+. We recommend the latest stable version:
```bash
rustup update
```

### Can I build without Cargo?

No. Cargo is the Rust build system and dependency manager. However, you can:
1. Use Docker for isolated builds
2. Cross-compile for different platforms
3. Use maturin for Python bindings (future)

## Configuration

### What are the recommended settings?

Use `MarsConfig::default()` for most cases. It's tuned for general reasoning tasks:

```rust
let config = MarsConfig::default();
```

For specific tasks:
- **Math/Logic**: Lower temperatures (0.1-0.5)
- **Creative**: Higher temperatures (0.8-1.5)
- **General**: Mixed (0.3-1.0)

### How do I adjust token budget?

```rust
let config = MarsConfig {
    token_budget: 5000,  // Set maximum tokens
    ..Default::default()
};
```

### Can I use different models for different phases?

Currently, MARS uses a single ModelClient. You can:
1. Wrap multiple clients in your ModelClient implementation
2. Route to different models internally
3. Use provider routing for model selection

## Usage

### How do I implement ModelClient?

```rust
use optillm_core::ModelClient;

pub struct MyClient;

impl ModelClient for MyClient {
    fn stream(&self, prompt: &Prompt)
        -> Pin<Box<dyn Stream<Item = Result<ResponseEvent, OptillmError>> + Send>> {
        // Implement streaming responses
    }
}
```

### What if my LLM provider is not supported?

optillm-rs is provider-agnostic. Implement `ModelClient` for any provider:
- OpenAI
- Anthropic
- Google
- Custom endpoints
- Local models

### How do I handle errors?

```rust
match coordinator.optimize(query, &client).await {
    Ok(result) => println!("Answer: {}", result.answer),
    Err(e) => eprintln!("Error: {}", e),
}
```

Errors include:
- `ExplorationError` - Agent failures
- `VerificationError` - Verification issues
- `AggregationError` - Synthesis problems
- `OptillmError` - Core errors

### Can I stream results?

MARS emits events during execution:
```rust
// Event streaming supported internally
// Access results via returned MarsOutput
```

## Performance

### Why is my optimization slow?

Factors affecting speed:
1. **Number of agents** - More agents = more parallel calls
2. **LLM latency** - Network/API latency dominates
3. **Token budget** - More tokens = longer responses
4. **Iterations** - More iterations = more refinement

### How do I optimize for latency?

```rust
let config = MarsConfig {
    num_agents: 3,        // Reduce agents
    max_iterations: 1,    // Fewer refinement rounds
    parallel_exploration: true,  // Parallel exploration
    ..Default::default()
};
```

### How do I optimize for cost?

```rust
let config = MarsConfig {
    num_agents: 3,        // Fewer agents
    token_budget: 2000,   // Set token limit
    max_iterations: 1,    // Single pass
    ..Default::default()
};
```

### How do I optimize for quality?

```rust
let config = MarsConfig {
    num_agents: 7,        // More agents
    token_budget: 10000,  // Higher token limit
    max_iterations: 3,    // More refinement
    ..Default::default()
};
```

## Strategies

### When should I use MOA?

Use Mixture of Agents when:
- ✅ Quality is critical
- ✅ Multiple perspectives help
- ✅ You have token budget
- ✅ Latency isn't critical

### When should I use MCTS?

Use Monte Carlo Tree Search when:
- ✅ Exploring reasoning trees
- ✅ You need low cost
- ✅ Sequential reasoning
- ✅ Dialogue systems

### When should I use MARS?

Use MARS when:
- ✅ Want best overall performance
- ✅ Complex reasoning needed
- ✅ Quality matters
- ✅ Can tolerate higher latency/cost

## Verification

### Why does verification fail?

Possible causes:
1. Model doesn't correctly evaluate answers
2. Solution actually incorrect
3. Verification criteria too strict
4. Ambiguous/subjective problem

### Can I use custom verification?

Currently, verification is built-in. You can:
1. Implement post-hoc verification
2. Adjust `verification_threshold`
3. Use different aggregation methods

### What's a good confidence threshold?

- **0.5**: Very permissive
- **0.7**: Balanced (recommended)
- **0.9**: Strict

## Strategy Network

### How does strategy learning work?

The strategy network tracks:
1. Successful solution patterns
2. Success rates for strategies
3. Agent contributions
4. Collective knowledge

### Can I pre-populate strategies?

Not yet. Future enhancement planned.

### Can I export strategies?

Currently strategies are in-memory. Future versions will support:
- Strategy export/import
- Persistence
- Sharing

## Troubleshooting

### Compilation errors

Common causes:
1. Old Rust version - `rustup update`
2. Missing dependencies - `cargo update`
3. Platform issues - Check specific setup for your OS

### Runtime panics

Always check:
1. Error handling in your code
2. Valid ModelClient implementation
3. Proper prompt construction
4. Resource availability

### Low quality results

Try:
1. Increase `num_agents`
2. Expand temperature range
3. Better system prompt
4. More iterations
5. Higher token budget

### High token usage

Monitor:
1. Number of agents
2. Response lengths
3. Iteration count
4. Verification depth

## Contributing

### How do I contribute?

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Write tests
5. Submit a pull request

See [Contributing Guide](development/contributing.md).

### How do I report bugs?

1. Check existing issues
2. Create detailed bug report
3. Include reproduction steps
4. Provide version info

### How do I suggest features?

1. Open an issue with label `enhancement`
2. Describe use case
3. Suggest implementation approach
4. Discuss with maintainers

## Performance Tuning

### What's the optimal team size?

For MARS:
- **Small** (3-5 agents): Fast, lower cost
- **Medium** (5-7 agents): Balanced (recommended)
- **Large** (7-10 agents): High quality, high cost

### How many iterations do I need?

- **1**: Single pass (fast)
- **2**: Most improvements captured (recommended)
- **3+**: Diminishing returns

### Should I use all features?

For production:
- ✅ Enable strategy learning
- ✅ Enable feedback loops
- ✅ Use appropriate verification
- ❌ Don't over-configure

## Benchmarking

### How do I benchmark?

```rust
use std::time::Instant;

let start = Instant::now();
let result = coordinator.optimize(query, &client).await?;
let elapsed = start.elapsed();

println!("Time: {:?}, Tokens: {}", elapsed, result.total_tokens);
```

### What should I measure?

1. **Latency** - Time to result
2. **Token usage** - Cost metric
3. **Quality** - Correctness/usefulness
4. **Throughput** - Queries per second

## Support

### Where can I get help?

1. Check this FAQ
2. Read the [documentation](index.md)
3. Search GitHub issues
4. Open a new issue
5. Ask in discussions

### Is there a community?

The community is growing! Check:
- GitHub Issues & Discussions
- Rust community forums
- This documentation

### Commercial support?

Not yet available. Stay tuned!

## License & Legal

### What license is optillm-rs under?

MIT License - Free for commercial and private use.

### Can I use optillm-rs commercially?

Yes! MIT license allows commercial use.

### Do I need to contribute back?

No, it's open source. Contributions are appreciated but not required.

---

**More questions?** [Open an issue](https://github.com/coohom/optillm-rs/issues) or check the full documentation.
