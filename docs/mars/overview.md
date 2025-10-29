# MARS: Multi-Agent Reasoning System

MARS is a production-ready implementation of the Multi-Agent Reasoning System that achieves significant improvements on complex reasoning tasks.

## Overview

MARS uses multiple AI agents with diverse temperature settings to explore different solution paths in parallel, then applies verification, aggregation, and iterative improvement to synthesize the best possible answer.

## Benchmark Results

| Task | Baseline | MARS | Improvement |
|------|----------|------|-------------|
| AIME 2025 | 43.3% | 73.3% | **+69%** |
| IMO 2025 | 16.7% | 33.3% | **+100%** |
| LiveCodeBench | 39.05% | 50.48% | **+29%** |

## Key Features

### 🤖 Multi-Agent Exploration
- Multiple agents explore diverse solution paths
- Configurable temperature ranges (default: 0.3, 0.6, 1.0)
- Parallel exploration for efficiency
- Independent reasoning from each agent

### ✅ Cross-Agent Verification
- Consensus-based verification
- Multiple verification methods
- Confidence scoring
- Quality assessment

### 🔗 Solution Aggregation
- RSA-inspired aggregation algorithm
- Multiple aggregation strategies
- Synthesis of best aspects from multiple solutions
- Intelligent combination logic

### 🔄 Iterative Improvement
- Multi-pass refinement
- Strategy extraction from successful approaches
- Pattern learning and application
- Feedback-driven enhancement

### 🧠 Strategy Network
- Collective learning across agents
- Success rate tracking
- Strategy sharing
- Knowledge preservation

## How MARS Works

### Phase 1: Exploration
```
Initialize N agents with diverse temperatures
↓
Each agent explores solution space independently
↓
Collect candidate solutions from all agents
↓
Store solutions with metadata
```

### Phase 2: Verification
```
For each solution:
  - Run verification checks
  - Compute confidence score
  - Compare with other solutions
  - Mark correctness
```

### Phase 3: Aggregation
```
Select top solutions based on verification
↓
Synthesize combined solution
↓
Blend best reasoning paths
↓
Create improved candidate
```

### Phase 4: Improvement
```
Extract patterns from successful solutions
↓
Learn new strategies
↓
Apply insights to refinement
↓
Iteratively enhance answer
```

### Phase 5: Selection
```
Evaluate all candidates
↓
Score based on:
  - Verification results
  - Reasoning quality
  - Confidence level
↓
Select best answer
↓
Return with metadata
```

## Configuration

### Default Configuration
```rust
let config = MarsConfig::default();
```

### Custom Configuration
```rust
let config = MarsConfig {
    num_agents: 5,
    temperatures: vec![0.3, 0.6, 0.9, 1.0, 1.2],
    max_iterations: 3,
    verification_threshold: 0.7,
    aggregation_method: AggregationMethod::RSA,
    enable_strategy_learning: true,
    enable_feedback_loops: true,
    parallel_exploration: true,
    token_budget: 10000,
};
```

## Temperature Effects

Different temperatures produce different types of reasoning:

| Temperature | Behavior | Use Case |
|-------------|----------|----------|
| 0.1 - 0.3 | Conservative, focused | Logic-heavy problems |
| 0.4 - 0.6 | Balanced | General reasoning |
| 0.7 - 1.0 | More varied | Creative tasks |
| 1.1 - 1.5 | Highly diverse | Brainstorming |

MARS combines these approaches to get complementary strengths.

## Usage

### Basic Usage
```rust
let config = MarsConfig::default();
let coordinator = MarsCoordinator::new(config);
let result = coordinator.optimize(query, &client).await?;
```

### With Custom Config
```rust
let config = MarsConfig {
    num_agents: 7,
    temperatures: vec![0.3, 0.5, 0.7, 0.9, 1.1, 1.3, 1.5],
    max_iterations: 5,
    verification_threshold: 0.8,
    ..Default::default()
};

let coordinator = MarsCoordinator::new(config);
let result = coordinator.optimize(query, &client).await?;
```

### Accessing Results
```rust
let result = coordinator.optimize(query, &client).await?;

println!("Answer: {}", result.answer);
println!("Reasoning: {}", result.reasoning);
println!("Total tokens: {}", result.total_tokens);
println!("Iterations: {}", result.iterations);
println!("All solutions: {:?}", result.all_solutions);
```

## Aggregation Methods

MARS supports multiple aggregation strategies:

### RSA (Reasoning-aware Synthesis)
- Analyzes reasoning paths
- Synthesizes best approach
- Default method

### Majority Voting
- Consensus-based selection
- Useful for factual questions
- Simple and interpretable

### Best-of-N
- Select highest scoring solution
- Minimal computation
- Baseline approach

### Mixture of Experts
- Weighted combination
- Expert weight learning
- Advanced approach

## Strategy Network

MARS includes a strategy network for learning:

```rust
let mut strategy_network = StrategyNetwork::new();

// Register discovered strategies
let strategy_id = strategy_network.register_strategy(
    "agent-1".to_string(),
    "Break into steps".to_string(),
    "Use chain-of-thought".to_string(),
);

// Track success
strategy_network.update_success_rate(&strategy_id, true)?;

// Get top strategies
let top = strategy_network.get_top_strategies(5);
```

## Verification

Verification ensures solution quality:

```rust
pub struct VerificationResult {
    pub solution_id: String,
    pub is_correct: bool,
    pub confidence_score: f32,
    pub verification_method: String,
    pub reasoning: String,
    pub created_at: DateTime<Utc>,
}
```

## Performance Optimization

### Token Efficiency
- Set `token_budget` to limit consumption
- Monitor `total_tokens` in results
- Adjust `num_agents` based on budget

### Latency Optimization
- Use parallel exploration (enabled by default)
- Reduce `num_agents` for faster results
- Limit `max_iterations`

### Quality Optimization
- Increase `num_agents` for diversity
- Adjust temperatures for the task
- Enable strategy learning for improvement
- Increase `max_iterations` for refinement

## Best Practices

### For Math/Logic Problems
```rust
let config = MarsConfig {
    num_agents: 5,
    temperatures: vec![0.1, 0.3, 0.5, 0.7, 0.9],
    max_iterations: 3,
    verification_threshold: 0.9,
    ..Default::default()
};
```

### For Creative Tasks
```rust
let config = MarsConfig {
    num_agents: 5,
    temperatures: vec![0.8, 1.0, 1.2, 1.4, 1.6],
    max_iterations: 2,
    verification_threshold: 0.5,
    ..Default::default()
};
```

### For General Use
```rust
let config = MarsConfig::default();  // Recommended defaults
```

## Monitoring and Debugging

### Enable Logging
```rust
env_logger::init();
// Set RUST_LOG=debug for detailed output
```

### Track Progress
```rust
let result = coordinator.optimize(query, &client).await?;

for solution in &result.all_solutions {
    println!("Agent: {}", solution.agent_id);
    println!("Temp: {}", solution.temperature);
    println!("Score: {:.2}", solution.verification_score);
    println!("Verified: {}", solution.is_verified);
}
```

### Analyze Verifications
```rust
for verification in &result.verifications {
    println!("Solution: {}", verification.solution_id);
    println!("Correct: {}", verification.is_correct);
    println!("Confidence: {:.2}", verification.confidence_score);
}
```

## Troubleshooting

### Low Quality Results
- Increase `num_agents`
- Expand temperature range
- Enable strategy learning
- Increase `max_iterations`

### High Token Usage
- Reduce `num_agents`
- Decrease `max_iterations`
- Set `token_budget`
- Use simpler verification

### Slow Execution
- Reduce `num_agents`
- Ensure `parallel_exploration: true`
- Decrease `max_iterations`
- Use faster model

## Advanced Configuration

For enterprise use cases:

```rust
let config = MarsConfig {
    num_agents: 10,
    temperatures: vec![0.1, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4, 1.6, 2.0],
    max_iterations: 5,
    verification_threshold: 0.85,
    aggregation_method: AggregationMethod::MixtureOfExperts,
    enable_strategy_learning: true,
    enable_feedback_loops: true,
    parallel_exploration: true,
    token_budget: 50000,
};
```

## See Also

- [MARS Configuration](configuration.md)
- [Verification](verification.md)
- [Aggregation](aggregation.md)
- [Strategy Network](strategy-network.md)
- [Benchmarks](benchmarks.md)
