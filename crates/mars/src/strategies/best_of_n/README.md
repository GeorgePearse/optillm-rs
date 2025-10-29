# Best-of-N Sampling Strategy

## Overview

The Best-of-N strategy is a straightforward but effective approach to improving LLM output quality. It generates N diverse candidate solutions with varying parameters (typically temperature), then selects the best one based on configurable selection criteria.

## Algorithm

1. **Generation Phase**: Generate N diverse solutions by sampling at different temperatures
2. **Selection Phase**: Evaluate all candidates using one of several selection methods
3. **Return**: The highest-scoring solution

## Configuration Options

### `BestOfNConfig`

- **`num_candidates`**: Number of solutions to generate (default: 5)
- **`temperatures`**: Temperature values for diversity (auto-generated or custom)
- **`selection_method`**: Criteria for choosing best solution
- **`use_verification_scores`**: Whether to use verification scores in selection (default: true)

### Selection Methods

- **`BestScore`**: Highest verification score
- **`HighestConfidence`**: Length-weighted quality heuristic
- **`MostThorough`**: Longest reasoning chain
- **`MostConcise`**: Shortest answer
- **`MultiCriteria`**: Weighted combination of all criteria (40% score, 30% thoroughness, 20% conciseness, 10% diversity)

## Usage Example

```rust
use optillm_mars::best_of_n::{BestOfNAggregator, BestOfNConfig, SelectionMethod};

let config = BestOfNConfig::new(10)
    .with_selection_method(SelectionMethod::MultiCriteria)
    .with_temperatures(vec![0.3, 0.5, 0.7, 0.9, 1.1]);

let (solution, metadata) = BestOfNAggregator::run_best_of_n(
    query,
    system_prompt,
    config,
    &client,
).await?;

println!("Best solution: {}", solution.answer);
println!("Selection score: {}", metadata.selection_score);
```

## When to Use

- **Simple tasks** where multiple attempts can find the right answer
- **Diverse solution spaces** where temperature sampling explores different approaches
- **Budget-conscious scenarios** where you want better quality without complex aggregation
- **Quick wins** when you need immediate improvement without tuning

## Performance Characteristics

- **Computational Cost**: Linear in N (generates N independent solutions)
- **Token Usage**: N × tokens per solution
- **Latency**: Can parallelize generation, latency ≈ 1 solution time
- **Quality Improvement**: Typically 10-30% over single-shot, depends on task

## References

Best-of-N is a standard technique in the LLM community, widely used for:
- Code generation (sampling multiple implementations)
- Mathematical reasoning (exploring different solution paths)
- Creative tasks (generating diverse options)

Related work:
- Li et al. (2022): "Large Language Models are Zero-Shot Reasoners"
- Cobbe et al. (2021): "Training Verifiers to Solve Math Word Problems"
