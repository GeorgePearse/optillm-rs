# Diverse Sampling

Diverse Sampling is an optimization strategy that generates multiple answers using systematically varied temperature levels to explore the response space with different levels of creativity and determinism, then selects the best response.

## Algorithm Overview

Diverse Sampling operates in two main phases:

### Phase 1: Temperature-Varied Generation
Generate N responses with temperatures linearly interpolated between min and max:
- **Low temperature samples** (e.g., 0.3): More focused, deterministic, factual
- **Mid temperature samples** (e.g., 0.9): Balanced between creativity and focus
- **High temperature samples** (e.g., 1.5): More exploratory, creative, varied

### Phase 2: Selection
Select the best response from the generated samples (currently selects first sample; can be enhanced with scoring).

## How It Works in Practice

### Example: Creative Writing Task

**Query:** "Write a short story about a lost time traveler"

**Temperature Distribution (5 samples):**
- Sample 1 (0.3°): Deterministic, straightforward narrative
  "The traveler appeared in 1955. He checked his device. It was broken..."

- Sample 2 (0.675°): Slightly more creative
  "A flickering figure materialized in a 1955 diner. The device sparked..."

- Sample 3 (1.05°): Balanced creativity
  "In a sudden burst of chrono-energy, the weary traveler found himself..."

- Sample 4 (1.425°): More imaginative
  "Reality twisted around the lost temporal explorer, depositing them in..."

- Sample 5 (1.5°): Maximum creativity
  "The very fabric of time rewove itself, scattering fragments of memory..."

**Selection:** Choose the best from these diverse approaches

### Example: Technical Problem

**Query:** "Explain quantum entanglement"

**Temperature Distribution:**
- 0.3°: Direct, textbook explanation
- 0.9°: More narrative explanation with analogy
- 1.5°: Creative metaphor-heavy explanation

**Benefit:** Get both technical and intuitive explanations to choose from

## When to Use Diverse Sampling

✅ **Best For:**
- Tasks benefiting from multiple perspectives
- Creative writing and generation
- Explanation and teaching tasks
- Open-ended problem solving
- Tasks where different answer styles are valuable
- Exploring the response space

❌ **Not Ideal For:**
- Time-critical applications (generates N samples)
- Tasks with single correct answer
- Real-time inference with latency constraints
- Very expensive API calls
- Tasks already optimized for speed

## Configuration

```rust
use optillm_mars::strategies::diverse_sampling::{DiverseSamplingConfig, DiverseSamplingAggregator};

// Default configuration (5 samples, 0.3-1.5 temperature)
let config = DiverseSamplingConfig::new();

// Custom configuration
let config = DiverseSamplingConfig::new()
    .with_num_samples(10)
    .with_min_temperature(0.2)
    .with_max_temperature(1.8)
    .with_max_tokens(2048);

// Manual configuration
let config = DiverseSamplingConfig {
    num_samples: 7,                    // Generate 7 samples
    min_temperature: 0.1,              // Start with very deterministic
    max_temperature: 1.9,              // End with very creative
    max_tokens: 3000,
};
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `num_samples` | 5 | Number of samples with different temperatures |
| `min_temperature` | 0.3 | Minimum temperature (deterministic) |
| `max_temperature` | 1.5 | Maximum temperature (creative) |
| `max_tokens` | 4096 | Maximum tokens per sample |

## Usage Example

```rust
use optillm_mars::strategies::diverse_sampling::{DiverseSamplingConfig, DiverseSamplingAggregator};
use optillm_core::ModelClient;

// Configure Diverse Sampling
let config = DiverseSamplingConfig::new()
    .with_num_samples(8)
    .with_min_temperature(0.2)
    .with_max_temperature(1.6);

// Run Diverse Sampling strategy
let result = DiverseSamplingAggregator::run_diverse_sampling(
    "Write a poem about stars",
    "You are a creative poet",
    config,
    &client
).await?;

println!("Best answer: {}", result.best_answer);
println!("Used temperature: {}", result.best_temperature);
println!("Total tokens: {}", result.metadata.total_tokens);
println!("Unique answers: {}", result.metadata.unique_answers);
```

## Performance Characteristics

### Token Usage
Per execution:
- 5 samples × average tokens per sample = Total tokens
- With 5 samples: ~5 × 1,500 = ~7,500 tokens
- Linear increase with number of samples

### Latency
- Sequential sampling: 5 × generation_latency
- Typical latency: ~15-40 seconds for 5 samples

### Quality Impact
- **Diversity:** High (explores response space)
- **Coverage:** Provides multiple perspectives
- **Improvement:** Depends on selection strategy

## Key Insights

### Temperature as Control Lever
Temperature is a fundamental parameter controlling:
- **Creativity:** Higher temp = more creative/diverse
- **Determinism:** Lower temp = more focused/consistent
- **Exploration:** Full range provides complete picture

### Why Diverse Sampling Works
1. **Multiple Perspectives:** Different temperatures reveal different aspects
2. **Coverage:** Explores both conservative and creative solutions
3. **Flexibility:** User can choose from diverse options
4. **Interpretability:** Clear temperature-to-style mapping

### Linear Temperature Distribution
Evenly spaced temperatures ensure:
- Smooth gradient from deterministic to creative
- No gaps in the response space
- Predictable behavior across the range

## Advanced Features

### Temperature Space Exploration
- **Narrow Range (0.3-0.5):** Conservative exploration
- **Wide Range (0.1-1.9):** Extreme diversity
- **Custom Range:** Tune to task requirements

### Sample Diversity Analysis
Metadata includes:
- `unique_answers`: Count of distinct responses
- Individual temperatures per sample
- Token count per sample

### Flexible Selection
Current implementation selects first sample; can be enhanced with:
- Scoring functions (length, complexity, novelty)
- Consensus voting
- Confidence-based selection
- User-guided selection

## Comparison with Other Strategies

| Strategy | Approach | Cost | Best For |
|----------|----------|------|----------|
| **Best-of-N** | Multiple attempts | Medium | Speed |
| **Self-Consistency** | Voting | High | Consensus |
| **Diverse Sampling** | Temperature variation | High | Exploration |
| **ReRead** | Re-reading | Very Low | Focus |
| **PVG** | Adversarial | Very High | Reasoning |

## Implementation Notes

- **Linear Interpolation:** Temperatures evenly distributed across range
- **Single Pass:** Each temperature gets one generation
- **Deterministic:** Same config produces same temperature sequence
- **Parallel Ready:** Samples could be generated in parallel

## Common Pitfalls

⚠️ **Too many samples:** Exponential cost increase
⚠️ **Conflicting temperature range:** Will fail validation
⚠️ **No selection strategy:** Current implementation just picks first
⚠️ **Inappropriate temperature range:** Task-dependent tuning needed
⚠️ **Expecting consistency:** High-temperature samples will vary
⚠️ **Token limits:** Very high temps need higher token limits

## References

**Related Concepts:**
- Temperature-controlled sampling in neural networks
- Softmax and probability distributions
- Multi-path exploration
- Sampling diversity and coverage

**Original Concept:**
- Temperature-based decoding variations
- Diverse beam search variants
- Multiple hypothesis generation

## When to Combine with Other Strategies

Diverse Sampling works well with:
- **Selection/Voting:** Vote on the most common answer
- **Scoring:** Score diversity across samples
- **Refinement:** Use best sample as base for further improvement
- **Analysis:** Analyze diversity of outputs for task understanding

## Practical Tips

1. **Start with defaults:** 5 samples with 0.3-1.5 works for most tasks
2. **Adjust for task:** Creative tasks need higher max temp
3. **Consider cost:** Each sample costs full generation
4. **Analyze results:** Look at unique_answers to understand diversity
5. **Combine strategies:** Pair with voting or scoring for better selection

## Future Enhancements

- **Intelligent Selection:** Score samples by quality metrics
- **Parallel Generation:** Generate samples concurrently
- **Adaptive Temperatures:** Adjust based on response variance
- **Consensus Finding:** Vote on common elements across samples
- **Quality Metrics:** Track answer length, complexity, coherence
- **Feedback Loop:** Adjust temperature range based on results

