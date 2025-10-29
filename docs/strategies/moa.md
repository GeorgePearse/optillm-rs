# Mixture of Agents (MOA)

MOA (Mixture of Agents) is a strategy that generates diverse completions and synthesizes them into a high-quality answer.

## Algorithm

The MOA strategy operates in three phases:

### Phase 1: Generate Diverse Completions
- Generate N completions with high temperature
- Encourage diversity in responses
- Collect all candidate solutions

### Phase 2: Critique
- Analyze each completion
- Identify strengths and weaknesses
- Generate structured critique

### Phase 3: Synthesize
- Use critiques to inform synthesis
- Generate final optimized answer
- Combine best aspects from all candidates

## Usage

### Basic MOA

```rust
use optillm_mars::MoaAggregator;

let (solution, metadata) = MoaAggregator::run_moa(
    query,
    system_prompt,
    3,  // number of completions
    true,  // fallback enabled
    &client,
).await?;

println!("Answer: {}", solution.answer);
println!("Reasoning: {}", solution.reasoning);
```

### With Custom Settings

```rust
let (solution, metadata) = MoaAggregator::run_moa(
    "Your question here",
    "You are a helpful assistant",
    5,  // Generate 5 diverse completions
    true,  // Enable fallback if completion fails
    &your_model_client,
).await?;

// Check metadata
println!("Phase 1 tokens: {}", metadata.phase1_tokens);
println!("Phase 2 tokens: {}", metadata.phase2_tokens);
println!("Phase 3 tokens: {}", metadata.phase3_tokens);
println!("Fallback used: {}", metadata.fallback_used);
```

## Metadata

MOA provides detailed metadata:

```rust
pub struct MoaMetadata {
    pub total_tokens: usize,
    pub phase1_tokens: usize,
    pub phase2_tokens: usize,
    pub phase3_tokens: usize,
    pub num_completions: usize,
    pub fallback_used: bool,
}
```

## Configuration

### Number of Completions
- **3** (default): Good balance of diversity and cost
- **2**: Minimal (faster, cheaper)
- **5+**: High diversity (more expensive)

### Fallback Settings
- **true** (recommended): Continue if one completion fails
- **false**: Strict mode, fail if any completion fails

## Example: Creative Writing

```rust
use optillm_mars::MoaAggregator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();

    let (solution, metadata) = MoaAggregator::run_moa(
        "Write a short story about a robot discovering emotions",
        "You are a creative science fiction writer.",
        5,  // Generate 5 diverse stories
        true,
        &client,
    ).await?;

    println!("Generated Story:\n{}", solution.answer);
    println!("\nToken Usage:");
    println!("  Phase 1 (Generation): {}", metadata.phase1_tokens);
    println!("  Phase 2 (Critique): {}", metadata.phase2_tokens);
    println!("  Phase 3 (Synthesis): {}", metadata.phase3_tokens);
    println!("  Total: {}", metadata.total_tokens);

    Ok(())
}
```

## Example: Technical Explanation

```rust
use optillm_mars::MoaAggregator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();

    let (solution, metadata) = MoaAggregator::run_moa(
        "Explain how neural networks work for beginners",
        "You are an expert at explaining complex topics simply.",
        3,
        true,
        &client,
    ).await?;

    println!("Best Explanation:\n{}", solution.answer);
    println!("Based on {} candidate responses", metadata.num_completions);

    Ok(())
}
```

## Strengths

✅ **Diversity**: Generates multiple perspectives
✅ **Quality**: Synthesis improves over individual responses
✅ **Interpretability**: Can understand reasoning from critiques
✅ **Fallback Support**: Handles partial failures

## Limitations

❌ **Cost**: 3x token usage (generation + critique + synthesis)
❌ **Latency**: Three sequential phases
❌ **Complexity**: Requires good critique ability

## When to Use

### Best For:
- Creative writing
- Multiple perspectives on a topic
- Complex explanations
- Nuanced problem-solving
- When quality is more important than cost

### Not Ideal For:
- Simple factual retrieval
- When latency is critical
- When cost is constrained
- Real-time applications

## Cost Optimization

### Reduce Completions
```rust
// Minimal mode
MoaAggregator::run_moa(..., 2, true, &client).await?
```

### Use Faster Model
```rust
// Use faster model for phases 1 and 2
// Use better model for phase 3
```

### Batch Processing
```rust
// Process multiple queries with same critique
let critiques = gather_critiques_batch(...).await?;
```

## Comparison with Other Strategies

| Strategy | MOA | MCTS | MARS |
|----------|-----|------|------|
| Diversity | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| Cost | Medium | Low | High |
| Latency | Medium | Low | High |
| Quality | High | Medium | Highest |
| Interpretability | High | Low | Medium |

## Advanced Usage

### Custom Critique Prompt
```rust
// Could be modified in future versions to accept
// custom critique prompts for domain-specific critiquing
```

### Parallel Synthesis
```rust
// Future enhancement: Generate critiques in parallel
// instead of sequential phases
```

## Integration with MARS

MOA can be used as an aggregation method within MARS:

```rust
use optillm_mars::{MarsConfig, AggregationMethod};

let config = MarsConfig {
    aggregation_method: AggregationMethod::MixtureOfAgents,
    ..Default::default()
};
```

## Troubleshooting

### Low Quality Synthesis
- Check if critique phase is effective
- Verify system prompt is appropriate
- Increase number of completions

### High Token Usage
- Reduce `num_completions` to 2
- Use faster model for phases 1-2
- Monitor `metadata.total_tokens`

### Fallback Being Used
- Check client error logs
- Verify system prompt is valid
- Ensure completions are non-empty

## See Also
- [MARS Integration](../mars/overview.md)
- [Provider Routing](provider-routing.md)
- [Aggregation Methods](../mars/aggregation.md)
