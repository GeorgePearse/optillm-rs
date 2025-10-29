# MARS Aggregation

Solution aggregation combines verified solutions into a refined final answer.

## Overview

Aggregation synthesizes the best aspects of multiple solutions:

- **Consensus Building**: Identify agreement across solutions
- **Best Practices**: Extract successful techniques
- **Error Correction**: Combine strengths to offset weaknesses
- **Quality Improvement**: Refine through synthesis

## Aggregation Process

1. **Collection**: Gather verified solutions
2. **Analysis**: Compare reasoning and answers
3. **Synthesis**: Create combined solution
4. **Validation**: Verify aggregated result

## Aggregator Component

```rust
pub struct Aggregator;

impl Aggregator {
    pub fn aggregate_best_of_n(
        query: &str,
        system_prompt: &str,
        config: BestOfNConfig,
        client: &dyn ModelClient,
    ) -> Result<Vec<Solution>> {
        // Select best solutions
        // Return as collection
    }

    pub fn aggregate_self_consistency(
        // Similar to best_of_n
    ) -> Result<Vec<Solution>> {
        // Consensus voting approach
    }
}
```

## Aggregation Strategies

### Best-of-N
Select the highest quality solution:

```
Solutions: [A (0.9), B (0.7), C (0.8)]
Result: A (best score)
```

### Self-Consistency
Majority vote on answer:

```
Solutions: [answer1, answer1, answer2]
Result: answer1 (2/3 consensus)
```

### Synthesis
Combine reasoning from multiple solutions:

```
Solutions: [
  "Step 1: ..., Step 2: ..., Answer: X",
  "Step 1: ..., Step 2: ..., Answer: X",
  "Step 1: ..., Step 2: ..., Answer: Y"
]
Result: Synthesized best reasoning + answer X (consensus)
```

## Decision Making

```rust
pub fn select_best_answer(solutions: &[Solution]) -> String {
    // Count answer frequency
    let mut counts: HashMap<String, usize> = HashMap::new();
    for solution in solutions {
        *counts.entry(solution.answer.clone()).or_insert(0) += 1;
    }

    // Return most common
    counts.into_iter()
        .max_by_key(|(_answer, count)| *count)
        .map(|(answer, _)| answer)
        .unwrap_or_default()
}
```

## Quality Metrics

After aggregation, track:

```rust
pub struct AggregationMetrics {
    pub consensus_strength: f32,      // 0.0-1.0
    pub solution_diversity: f32,      // 0.0-1.0
    pub average_verification_score: f32,
    pub final_answer_confidence: f32, // 0.0-1.0
}
```

## Configuration

Aggregation behavior is controlled via config:

```rust
pub struct AggregationConfig {
    pub strategy: AggregationStrategy,
    pub min_consensus_threshold: f32,
    pub weight_verification_scores: bool,
}
```

## Iterative Aggregation

Multiple rounds of aggregation:

```
Round 1: Aggregate solutions -> result1
Round 2: Use result1 as seed + new solutions -> result2
Round 3: Use result2 as seed + new solutions -> final
```

## Performance

- **Consensus**: Slower but higher quality
- **Best-of-N**: Faster but may miss insights
- **Synthesis**: Slower, highest quality

See [MARS Overview](overview.md) and [Strategy Network](strategy-network.md).
