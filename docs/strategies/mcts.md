# Monte Carlo Tree Search (MCTS)

Monte Carlo Tree Search is a strategy for exploring solution space systematically.

## Overview

MCTS uses:

- **Tree Structure**: Represents problem states
- **Exploration**: UCB-based node selection
- **Simulation**: Random playouts from nodes
- **Backpropagation**: Update statistics

## When to Use MCTS

- Complex multi-step problems
- Game-like reasoning tasks
- Dialogue systems
- Exploration-exploitation tradeoff needed

## Configuration

```rust
pub struct MCTSConfig {
    pub num_iterations: usize,
    pub exploration_constant: f32,  // UCB parameter
    pub max_depth: usize,
    pub simulation_rollouts: usize,
}
```

## Example

```rust
let config = MCTSConfig::default();
let mcts = MCTS::new(config);

let result = mcts.optimize(query, client).await?;
```

## Implementation Details

See [MOA](moa.md) and [Best-of-N](best-of-n.md) for complementary strategies.
