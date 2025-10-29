# MARS Agent System

The agent system is the core of MARS, consisting of multiple agents with diverse exploration strategies.

## Overview

MARS agents are independent reasoning units that explore solution space with different temperatures:

- **Agent 0**: Low temperature (0.3) - Focused, deterministic
- **Agent 1**: Medium temperature (0.6) - Balanced exploration
- **Agent 2**: High temperature (1.0) - Creative, diverse

## Agent Structure

Each agent contains:

```rust
pub struct Agent {
    pub id: String,
    pub temperature: f32,
}
```

## Agent Lifecycle

1. **Initialization**: Create agents with temperature settings
2. **Solution Generation**: Each agent generates solutions
3. **Reasoning**: Agents provide step-by-step reasoning
4. **Verification**: Solutions are cross-verified
5. **Improvement**: Agents refine based on feedback

## Core Methods

### generate_solution_with_client()

Generate initial solution from query.

```rust
let solution = agent.generate_solution_with_client(
    query,
    use_thinking_tags,
    client,
).await?;
```

### verify_solution_with_client()

Cross-verify another agent's solution.

```rust
let score = agent.verify_solution_with_client(
    solution,
    client,
).await?;
```

### improve_solution_with_client()

Refine solution based on feedback.

```rust
let improved = agent.improve_solution_with_client(
    solution,
    feedback,
    use_thinking_tags,
    client,
).await?;
```

### extract_strategies_with_client()

Extract useful strategies from solution.

```rust
let strategies = agent.extract_strategies_with_client(
    solution,
    client,
).await?;
```

## Temperature Effects

Temperature controls exploration vs. exploitation:

### Low Temperature (0.3)
- Deterministic reasoning
- Focused on likely solutions
- Good for well-defined problems

### Medium Temperature (0.6)
- Balanced exploration
- Mix of standard and novel approaches
- Suitable for most problems

### High Temperature (1.0)
- Creative exploration
- Considers unconventional solutions
- Good for open-ended problems

## Solution Quality

Agents produce:

1. **Reasoning**: Step-by-step explanation
2. **Answer**: Final answer to the question
3. **Metadata**: Token usage, processing details

## Cross-Agent Dynamics

- **Verification**: Agents verify each other
- **Learning**: Strategies extracted from best solutions
- **Refinement**: Feedback improves future generations
- **Diversity**: Different temperatures avoid local optima

## Agent Failures

If an agent fails:

```rust
match agent.generate_solution_with_client(...).await {
    Ok(solution) => { /* process solution */ }
    Err(e) => {
        eprintln!("Agent {} failed: {}", agent.id, e);
        // Coordinator handles gracefully
    }
}
```

## Performance Considerations

- **Parallelism**: Agents run in parallel
- **Token Budget**: Respect per-agent token limits
- **Latency**: Number of agents affects total latency
- **Quality**: More agents generally improve quality

See [MARS Overview](overview.md) for system architecture.
