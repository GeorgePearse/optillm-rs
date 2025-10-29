# Self-Consistency Strategy

## Overview

Self-Consistency is a powerful technique that improves reasoning accuracy by generating multiple diverse reasoning paths and selecting the answer that appears most frequently through consensus voting. The key insight is that when different reasoning paths arrive at the same answer, that answer is highly likely to be correct.

## Algorithm

1. **Diverse Generation**: Generate K independent reasoning paths using different temperatures
2. **Answer Extraction**: Extract the final answer from each reasoning path
3. **Consensus Voting**: Use voting strategy to determine the most reliable answer
4. **Synthesis**: Combine insights from multiple paths into final solution

## Configuration Options

### `SelfConsistencyConfig`

- **`num_paths`**: Number of diverse reasoning paths to generate (default: 5)
- **`temperatures`**: Temperature values for diversity (auto-generated or custom)
- **`extraction_strategy`**: How to extract answers from reasoning text
- **`voting_strategy`**: Method for consensus voting
- **`consensus_threshold`**: Minimum agreement required (0.0-1.0, default: 0.5)
- **`weight_by_quality`**: Whether to weight votes by reasoning quality (default: true)

### Answer Extraction Strategies

- **`LastLine`**: Extract the final non-empty line
- **`AfterMarker`**: Extract text after "Answer:" or similar markers
- **`LastSentence`**: Extract the final sentence
- **`InQuotes`**: Extract text in quotation marks
- **`FullResponse`**: Use entire response as answer

### Voting Strategies

- **`MajorityVote`**: Simple majority (most common answer wins)
- **`QualityWeighted`**: Weight votes by reasoning length/quality
- **`HighestConfidence`**: Combine vote count with reasoning quality
- **`RankedChoice`**: Instant-runoff voting based on first appearance and frequency

## Usage Example

```rust
use optillm_mars::self_consistency::{
    SelfConsistencyAggregator, SelfConsistencyConfig,
    AnswerExtractionStrategy, VotingStrategy
};

let config = SelfConsistencyConfig::new(10)
    .with_extraction_strategy(AnswerExtractionStrategy::AfterMarker)
    .with_voting_strategy(VotingStrategy::QualityWeighted)
    .with_consensus_threshold(0.7);

let (solution, metadata) = SelfConsistencyAggregator::run_self_consistency(
    query,
    system_prompt,
    config,
    &client,
).await?;

println!("Consensus answer: {}", solution.answer);
println!("Agreement: {:.1}%", metadata.consensus_score * 100.0);
println!("Unique answers: {}", metadata.voting_results.len());
```

## When to Use

- **Mathematical reasoning** where multiple approaches can solve the same problem
- **Logic puzzles** with definite right/wrong answers
- **Tasks with verifiable outputs** where correctness can be determined
- **High-stakes decisions** where reliability is critical
- **Scenarios with clear answer formats** (numbers, yes/no, multiple choice)

## Performance Characteristics

- **Computational Cost**: Linear in K (generates K independent paths)
- **Token Usage**: K × tokens per path
- **Latency**: Can parallelize generation, latency ≈ 1 path time
- **Quality Improvement**: 15-40% improvement over single-shot, especially on reasoning tasks
- **Reliability**: Higher consensus score indicates greater confidence in answer

## Key Insights

1. **Diversity is crucial**: Use varied temperatures to explore different reasoning approaches
2. **Convergence signals correctness**: When multiple paths agree, confidence is high
3. **Quality weighting helps**: Longer, more thorough reasoning often indicates better answers
4. **Works best with structured answers**: Clear answer formats improve extraction accuracy

## References

- **Wei et al. (2022)**: "Self-Consistency Improves Chain of Thought Reasoning in Language Models"
  - Paper: https://arxiv.org/abs/2203.11171
  - Shows 15+ point improvements on GSM8K, SVAMP, and other benchmarks
  - Introduces the core self-consistency with CoT paradigm

- **Wang et al. (2022)**: "Towards Reasoning in Large Language Models: A Survey"
  - Discusses self-consistency as a key technique for reliable reasoning

## Implementation Details

This implementation supports:
- Multiple answer extraction strategies for different response formats
- Quality-weighted voting to prioritize thorough reasoning
- Consensus scoring to measure answer reliability
- Configurable thresholds for high-confidence scenarios
- Detailed voting statistics and path metadata
