# MCTS (Monte Carlo Tree Search) Strategy

## Overview

MCTS applies tree search algorithms to LLM reasoning, systematically exploring the solution space by building a search tree. Each node represents a partial solution, and the algorithm balances exploration (trying new approaches) and exploitation (refining promising paths) to find high-quality solutions.

## Algorithm

1. **Selection**: Start from root, select child nodes using UCB1 formula balancing exploration/exploitation
2. **Expansion**: When reaching a leaf, generate new child solutions
3. **Simulation**: Evaluate the solution quality (via LLM or heuristic)
4. **Backpropagation**: Update visit counts and values up the tree
5. **Iteration**: Repeat for N simulations
6. **Best Path**: Extract the highest-value path from root to leaf

## Configuration Options

### `MCTSConfig`

- **`num_simulations`**: Number of tree search iterations (default: 20)
- **`exploration_constant`**: UCB1 exploration parameter (default: 1.414)
- **`max_depth`**: Maximum tree depth (default: 5)
- **`branching_factor`**: Children per node (default: 3)
- **`temperature_schedule`**: Temperature values by depth
- **`use_pruning`**: Whether to prune low-value branches (default: true)

### UCB1 (Upper Confidence Bound)

The selection formula balances exploitation and exploration:

```
UCB1(node) = avg_value + C * sqrt(ln(parent_visits) / node_visits)
```

- Higher `avg_value`: Exploit known good paths
- Higher `sqrt(...)` term: Explore less-visited nodes
- `C` (exploration_constant): Tunes the balance (higher = more exploration)

## Usage Example

```rust
use optillm_mars::mcts::{MCTSAggregator, MCTSConfig};

let config = MCTSConfig::new()
    .with_num_simulations(50)
    .with_exploration_constant(1.5)
    .with_max_depth(4)
    .with_branching_factor(3);

let (solution, metadata) = MCTSAggregator::run_mcts(
    query,
    system_prompt,
    config,
    &client,
).await?;

println!("Best solution: {}", solution.answer);
println!("Tree nodes: {}", metadata.total_nodes);
println!("Best path depth: {}", metadata.best_path_depth);
```

## When to Use

- **Complex reasoning tasks** with multiple solution approaches
- **Multi-step problems** where partial solutions can be evaluated
- **Search problems** with clear intermediate states
- **Planning tasks** requiring decision trees
- **Competitive tasks** (games, optimization) where exploration helps
- **High-value problems** justifying significant compute

## Performance Characteristics

- **Computational Cost**: O(N × B × D) where N=simulations, B=branching, D=depth
- **Token Usage**: Can be high (N × B × D tokens), but pruning helps
- **Latency**: Sequential by nature (simulations depend on backprop)
- **Quality Improvement**: 30-60% on search/planning tasks, variable on others
- **Memory**: Scales with tree size (B^D nodes in worst case)

## Key Parameters

### Exploration Constant (C)
- **Low (0.5-1.0)**: Exploit known good paths, faster convergence
- **Medium (1.4-2.0)**: Balanced exploration/exploitation (recommended)
- **High (2.5+)**: More exploration, better for novel problems

### Branching Factor
- **2-3**: Focused search, lower cost
- **4-5**: More diverse exploration
- **6+**: Very expensive, rarely needed

### Simulations
- **10-20**: Quick exploration, may miss optimal paths
- **30-50**: Good balance for most tasks
- **100+**: Thorough search for high-stakes problems

## Advanced Features

- **Progressive widening**: Increase branching factor with visit count
- **Virtual loss**: Parallelize simulations by pessimistically marking nodes
- **RAVE (Rapid Action Value Estimation)**: Share information across similar nodes
- **Transposition tables**: Reuse evaluations for equivalent states

## References

- **Original Paper**: Coulom, R. (2006). "Efficient Selectivity and Backup Operators in Monte-Carlo Tree Search"
- **LLM Applications**: Hao et al. (2024). "Reasoning with Language Model is Planning with World Model"
  - Paper: https://arxiv.org/abs/2405.00451
  - Applies MCTS to LLM reasoning, shows improvements on GSM8K and others

- **AlphaGo**: Silver et al. (2016). Demonstrated MCTS power in game playing
- **MuZero**: Schrittwieser et al. (2020). MCTS without explicit search space model

## Implementation Notes

This implementation:
- Uses UCB1 for node selection
- Supports depth-limited search
- Includes pruning for efficiency
- Provides detailed tree statistics
- Can extract full reasoning path from root to best leaf
