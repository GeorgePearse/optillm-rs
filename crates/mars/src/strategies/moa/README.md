# MOA (Mixture of Agents) Strategy

## Overview

Mixture of Agents leverages multiple different LLM models working together in a collaborative framework. Different models have different strengths - some excel at reasoning, others at creativity, others at factual accuracy. MOA combines these strengths by having agents propose solutions, then using an aggregator agent to synthesize the best final answer.

## Algorithm

1. **Diverse Proposals**: Multiple LLM agents generate independent solutions
2. **Agent Specialization**: Each agent can use different models, prompts, or parameters
3. **Aggregation**: An aggregator agent reviews all proposals and synthesizes the best answer
4. **Synthesis**: The aggregator combines insights from multiple perspectives

## Configuration Options

### `MoaConfig`

- **`proposer_models`**: List of LLM models to use as proposers
- **`aggregator_model`**: Model to use for final synthesis
- **`num_proposals_per_model`**: Solutions per proposer (default: 1)
- **`aggregation_strategy`**: How to combine proposals
- **`use_specialized_prompts`**: Whether each model gets custom prompts (default: false)

### Aggregation Strategies

- **`Synthesis`**: Aggregator reads all proposals and creates new answer
- **`Selection`**: Aggregator picks the single best proposal
- **`Voting`**: Aggregator facilitates majority vote among proposals
- **`Fusion`**: Aggregator merges compatible elements from multiple proposals

## Usage Example

```rust
use optillm_mars::moa::{MoaAggregator, MoaConfig};

let config = MoaConfig::new()
    .with_proposer_models(vec![
        "gpt-4".to_string(),
        "claude-3-opus".to_string(),
        "gemini-pro".to_string(),
    ])
    .with_aggregator_model("gpt-4".to_string())
    .with_num_proposals_per_model(2);

let (solution, metadata) = MoaAggregator::run_moa(
    query,
    system_prompt,
    config,
    &router,  // ModelClientRouter
).await?;

println!("Synthesized answer: {}", solution.answer);
println!("Proposers: {}", metadata.num_proposers);
println!("Total proposals: {}", metadata.all_proposals.len());
```

## When to Use

- **Multi-model access**: You have API keys for multiple LLM providers
- **Diverse perspectives**: Task benefits from different reasoning approaches
- **Quality critical**: Need highest possible quality, cost is secondary
- **Evaluation tasks**: Aggregator can judge/compare proposals objectively
- **Complementary strengths**: Different models have different capabilities

## Performance Characteristics

- **Computational Cost**: (N models × P proposals) + 1 aggregation call
- **Token Usage**: High (each proposer + aggregator reads all proposals)
- **Latency**: Can parallelize proposers; aggregation is sequential
- **Quality Improvement**: 20-40% over best single model on complex tasks
- **Cost**: Most expensive strategy (multiple model calls)

## Model Selection Tips

### Proposer Models
- **Reasoning-focused**: GPT-4, Claude, Gemini for logic/math
- **Creative**: Claude, GPT-4 for open-ended generation
- **Factual**: GPT-4, Gemini for knowledge-intensive tasks
- **Code**: GPT-4, Claude, Codex for programming
- **Mix strengths**: Combine complementary models

### Aggregator Model
- **Strong comprehension**: Must understand all proposals
- **Synthesis ability**: Should create novel combinations
- **Judgment**: Must evaluate quality objectively
- **Typically**: Use strongest model available (GPT-4, Claude Opus)

## Key Insights

1. **Diversity matters**: More diverse models = better proposals
2. **Aggregator is critical**: Weak aggregator wastes good proposals
3. **Diminishing returns**: 3-4 models usually optimal, more adds cost without much gain
4. **Specialization helps**: Customize prompts per model's strengths
5. **Cost vs quality**: Only use when quality justifies multi-model expense

## Example Configurations

### Balanced Quality/Cost
```rust
proposers: ["gpt-4", "claude-3-sonnet", "gemini-pro"]
aggregator: "gpt-4"
proposals_per_model: 1
```

### Maximum Quality
```rust
proposers: ["gpt-4", "claude-3-opus", "gemini-ultra", "grok"]
aggregator: "claude-3-opus"
proposals_per_model: 2
```

### Budget Conscious
```rust
proposers: ["gpt-3.5-turbo", "claude-3-haiku", "gemini-pro"]
aggregator: "gpt-4"
proposals_per_model: 1
```

## References

- **Wang et al. (2024)**: "Mixture-of-Agents Enhances Large Language Model Capabilities"
  - Paper: https://arxiv.org/abs/2406.04692
  - Introduces MOA framework, shows improvements on AlpacaEval and MT-Bench

- **Related Work**:
  - **Ensemble methods** in ML (boosting, bagging)
  - **Multi-model systems** (HELM benchmarks)
  - **Constitutional AI** (multi-stage refinement)

## Implementation Notes

This implementation:
- Supports arbitrary number of proposer models
- Can use different models for proposers and aggregator
- Provides detailed proposal metadata
- Tracks token usage per model
- Supports specialized prompting per model
- Can parallelize proposer generation
