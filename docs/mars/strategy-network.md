# MARS Strategy Network

The strategy network enables collective learning from solutions.

## Overview

The strategy network:

- **Extracts** successful reasoning patterns
- **Shares** strategies between agents
- **Learns** from verified solutions
- **Improves** future generations

## Strategy Extraction

From a good solution, extract:

```rust
pub async fn extract_strategies(
    solution: &Solution,
    client: &dyn ModelClient,
) -> Result<Vec<String>> {
    // Identify key techniques
    // List step-by-step approaches
    // Extract domain-specific patterns
}
```

Example extraction:

```
Solution: "First, decompose into subproblems. Then solve each
          independently. Finally, combine results."

Extracted Strategies:
  1. Decompose complex problems into subproblems
  2. Solve subproblems independently
  3. Combine results systematically
```

## Strategy Integration

Strategies are integrated into system prompts for future agents:

```
Base System Prompt:
  "You are a helpful assistant."

Enhanced with Strategies:
  "You are a helpful assistant. When solving problems:
   - Decompose complex problems into subproblems
   - Solve subproblems independently
   - Combine results systematically"
```

## Strategy Network Architecture

```
Agent 1 Solution
    ↓
Strategy Extraction
    ↓
Extracted Strategies
    ↓
Agent 2 & 3 System Prompt
    ↓
Improved Solutions
```

## Collective Learning

Over iterations:

- **Round 1**: Agents generate diverse solutions
- **Round 2**: Extract strategies from best solutions
- **Round 3**: New agents use extracted strategies
- **Round 4**: Further refinement

## Configuration

Strategy network behavior is configured via:

```rust
pub struct StrategyNetworkConfig {
    pub enabled: bool,
    pub max_strategies_per_round: usize,
    pub strategy_weight: f32,  // 0.0-1.0
}
```

## Performance Impact

**With Strategy Network:**
- Better solutions over time
- Cumulative improvement
- Longer overall time

**Without Strategy Network:**
- Faster individual rounds
- No cumulative improvement
- Simpler system

## Limitations

- Some strategies may not generalize
- Over-specialization risk
- Added latency from extraction

## Best Practices

1. **Verify before using**: Only use strategies from verified solutions
2. **Diversity**: Keep diverse strategies, not just best
3. **Rotation**: Retire old strategies periodically
4. **Validation**: Test strategies on diverse problems

See [MARS Overview](overview.md) and [Agent System](agent-system.md).
