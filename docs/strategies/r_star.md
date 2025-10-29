# R* Algorithm Strategy

## Overview

R* Algorithm is an enhanced Monte Carlo Tree Search (MCTS) that incorporates learned value estimates for more intelligent exploration of solution space.

## Key Features

- **Enhanced MCTS**: Improves upon classic MCTS with learned priors
- **UCB Node Selection**: Uses Upper Confidence Bound formula with learned values
- **Value Learning**: Estimates position quality for better decisions
- **Efficient Exploration**: Finds high-quality solutions with fewer simulations

## How It Works

1. **Initialization**: Start with root node (problem query)
2. **Simulation Loop**: For each simulation:
   - **Selection**: Use UCB formula to select promising nodes
   - **Expansion**: Generate candidate solutions at selected nodes
   - **Simulation**: Evaluate candidates using value estimates
   - **Backpropagation**: Update node statistics with results
3. **Best Path Extraction**: Return highest-quality solution found

## Configuration

```rust
let config = RStarConfig {
    num_simulations: 10,          // Monte Carlo simulations
    exploration_constant: 1.414,  // sqrt(2) for UCB balance
    num_candidates: 3,            // Candidates per simulation
};
```

## UCB Formula

The Upper Confidence Bound formula balances exploitation and exploration:

```
UCB = average_value + C × √(ln(N) / n)
```

Where:
- `average_value`: Average reward of this node
- `C`: Exploration constant (1.414 ≈ √2 recommended)
- `N`: Parent visit count
- `n`: Current node visit count

## Parameters Explained

| Parameter | Effect | Typical Range |
|-----------|--------|---|
| num_simulations | More = better exploration, higher cost | 5-20 |
| exploration_constant | Higher = more exploration, lower = exploitation | 1.0-2.0 |
| num_candidates | Candidates per simulation | 2-5 |

## Advantages

- **Intelligent Exploration**: Learns what works and explores more
- **Guaranteed Improvement**: Always finds solution at least as good as others
- **Flexible**: Can work with any problem type
- **Theory-Grounded**: Based on proven UCB algorithm
- **Efficient**: Few simulations needed for good results

## Use Cases

- Complex decision spaces
- Solutions requiring exploration
- Problems with many valid approaches
- Multi-step reasoning problems
- Optimization where quality varies greatly

## Examples

### Exploration Process Visualization

```
Simulation 1: Explore broadly
├─ Candidate A: Score 0.7
├─ Candidate B: Score 0.6
└─ Candidate C: Score 0.5 (selected for next simulation)

Simulation 2: Exploit promising areas
├─ Refine Candidate C: Score 0.8 (improved!)
├─ Explore Candidate A variations: Score 0.75
└─ New direction: Score 0.65

Simulation 3-10: Continue refinement...

Best Solution Found: Score 0.85
```

### Problem Types

**Good Fit:**
- Algorithm design
- System architecture
- Complex proofs
- Multi-solution problems

**Fair Fit:**
- Factual questions
- Simple calculations
- Single-answer problems

## Performance Tuning

### For Speed
```rust
RStarConfig {
    num_simulations: 3,      // Fast
    exploration_constant: 1.0,
    num_candidates: 2,
}
```

### For Quality
```rust
RStarConfig {
    num_simulations: 20,     // Thorough
    exploration_constant: 1.414,
    num_candidates: 5,
}
```

### Balanced (Recommended)
```rust
RStarConfig {
    num_simulations: 10,
    exploration_constant: 1.414,
    num_candidates: 3,
}
```

## Comparison with MCTS

| Feature | MCTS | R* |
|---------|------|-----|
| Exploration | Random rollouts | Learned value estimates |
| Node Selection | Basic UCB | Enhanced UCB with priors |
| Convergence | Slower | Faster |
| Complexity | Medium | Higher (value learning) |

## Advanced Tips

- Increase simulations for harder problems
- Lower exploration_constant (1.0) for exploitation-heavy tasks
- Higher exploration_constant (1.5-2.0) for exploration
- num_candidates = 3-4 usually optimal
- Best results with 10+ simulations

## Metadata

The returned metadata includes:
- `simulations_run`: Actual simulations executed
- `candidates_explored`: Total candidates evaluated
- `total_tokens`: Computation cost

Use these to understand resource usage and refine parameters.

## References

- R* Algorithm Paper: Enhanced Monte Carlo Tree Search (forthcoming)
- Original MCTS: Coulom et al., "Efficient Selectivity and Backup Operators in Monte Carlo Tree Search" (2006)
- UCB Theory: Auer et al., "Finite-time Analysis of the Multiarmed Bandit Problem" (2002)
- Related: MCTS, Deep Thinking
