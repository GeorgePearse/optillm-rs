# Round Trip Optimization (RTO)

Round Trip Optimization is a strategy that improves solution quality through a multi-step round-trip process. It generates multiple code/solution variations and synthesizes them into an optimized final version.

## Algorithm Overview

RTO works through four key steps:

### Step 1: Initial Generation (C1)
Generate the initial solution directly from the original query using a low temperature (deterministic).

### Step 2: Abstraction (Q2)
Ask the model to describe/summarize the initial solution as a set of instructions, capturing the high-level intent and approach.

### Step 3: Round-Trip Generation (C2)
Generate a new solution from the description/instructions alone. This forces the model to re-implement based on understanding rather than copying.

### Step 4: Synthesis (C3) - Conditional
- If C1 and C2 are identical (after normalization), use C1 as the final answer
- If they differ, synthesize them into an optimized final version (C3)

## When to Use RTO

✅ **Best For:**
- Code generation tasks where multiple valid implementations exist
- Problems requiring cross-verification of solutions
- Scenarios where re-implementing from scratch improves quality
- Code optimization and refinement

❌ **Not Ideal For:**
- Simple questions with single correct answers
- Tasks where description quality is poor
- Real-time applications (3-4 model calls per query)
- Constrained token budgets

## Configuration

```rust
use optillm_mars::strategies::rto::{RTOConfig, RTOAggregator};

// Default configuration (low temperature for deterministic output)
let config = RTOConfig::new();

// Custom temperatures for each phase
let config = RTOConfig::new()
    .with_initial_temperature(0.1)
    .with_all_temperatures(0.2);  // Set all phases to same temp

// Custom token limits
let config = RTOConfig {
    initial_temperature: 0.1,
    description_temperature: 0.1,
    second_temperature: 0.1,
    synthesis_temperature: 0.1,
    max_tokens_initial: 4096,
    max_tokens_description: 1024,
    max_tokens_second: 4096,
    max_tokens_synthesis: 4096,
};
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `initial_temperature` | 0.1 | Temperature for C1 generation |
| `description_temperature` | 0.1 | Temperature for Q2 abstraction |
| `second_temperature` | 0.1 | Temperature for C2 round-trip generation |
| `synthesis_temperature` | 0.1 | Temperature for C3 synthesis |
| `max_tokens_initial` | 4096 | Max tokens for C1 |
| `max_tokens_description` | 1024 | Max tokens for Q2 |
| `max_tokens_second` | 4096 | Max tokens for C2 |
| `max_tokens_synthesis` | 4096 | Max tokens for C3 |

## Usage Example

```rust
use optillm_mars::strategies::rto::{RTOConfig, RTOAggregator};
use optillm_core::ModelClient;

// Configure RTO
let config = RTOConfig::new();

// Run RTO strategy
let (solution, metadata) = RTOAggregator::run_rto(
    "Write a function to sort an array",
    "You are an expert programmer",
    config,
    &client
).await?;

println!("Final answer: {}", solution.answer);
println!("C1 == C2: {}", !metadata.solutions_differed);
println!("Total tokens: {}", metadata.total_tokens);
```

## How It Works in Detail

### Example: Function Generation

**Query:** "Write a function to reverse a string"

**Phase 1: Initial Generation (C1)**
```rust
fn reverse_string(s: &str) -> String {
    s.chars().rev().collect()
}
```

**Phase 2: Abstraction (Q2)**
```
Instruction: Create a function that takes a string and returns
a new string with characters in reverse order, using iterator
methods to collect them.
```

**Phase 3: Round-Trip (C2)**
```rust
fn reverse_string(s: &str) -> String {
    s.chars().rev().collect()
}
```

**Phase 4: Result**
Since C1 == C2, use C1. Otherwise synthesize if different.

## Performance Characteristics

### Token Usage
- **Best case (identical solutions):** 3 API calls
  - C1: ~500 tokens
  - Q2: ~200 tokens
  - C2: ~500 tokens
  - **Total: ~1,200 tokens**

- **Worst case (different solutions):** 4 API calls
  - All above + C3 synthesis: ~500 tokens
  - **Total: ~1,700 tokens**

### Latency
- Sequential: 4 × model_latency
- Typical: 2-5 minutes for complex code

### Quality Impact
- **Best For:** Multi-step solutions, complex logic
- **Improvement:** +10-20% quality improvement on code generation
- **Risk:** Description quality affects C2 generation

## Advanced Features

### Whitespace Normalization
Solutions are compared with whitespace normalization, so minor formatting differences don't trigger synthesis.

### Code Block Extraction
The strategy automatically extracts code from markdown code blocks:
```
Input: "Here's the code:\n```python\ncode here\n```"
Extracted: "code here"
```

### Graceful Fallback
If C1 and C2 are identical, the strategy returns C1 directly without synthesis, saving tokens.

## Comparison with Other Strategies

| Strategy | Approach | Cost | Quality |
|----------|----------|------|---------|
| **Best-of-N** | Multiple attempts | Medium | Good |
| **Self-Consistency** | Consensus voting | High | Very Good |
| **RTO** | Round-trip refinement | High | Very Good |
| **CoT Reflection** | Reasoning + reflection | Low | Good |

RTO is most similar to Self-Consistency but uses a different mechanism (round-trip vs consensus).

## Implementation Notes

- Uses regex for code block extraction (handles multiple languages)
- Low temperature (0.1) ensures deterministic outputs suitable for comparison
- Whitespace-normalized comparison prevents false differences
- Metadata includes all intermediate solutions for analysis

## References

**Related Papers:**
- "Let Models Explain Themselves" - Understanding model reasoning
- Self-verification and round-trip mechanisms for code quality
- Iterative refinement techniques in program synthesis

**Original Implementation:**
- Python optillm: `optillm/rto.py`
- Applicable to code generation, mathematics, and structured reasoning

## Common Pitfalls

⚠️ **High temperature during abstraction** - May cause Q2 to lose important details
⚠️ **Short max_tokens_description** - Q2 may be truncated, affecting C2 quality
⚠️ **Non-code tasks** - RTO works best with structured, generative tasks
⚠️ **Comparing to wrong baseline** - Account for extra tokens when benchmarking
