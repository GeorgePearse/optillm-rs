# PlanSearch

PlanSearch is an optimization strategy that solves complex problems through a structured planning approach: generating observations, deriving insights, and using them to create both natural language and code solutions.

## Algorithm Overview

PlanSearch works through four main steps:

### Step 1: Generate Initial Observations
Analyze the problem and generate several useful, non-obvious observations that serve as hints to solve the problem. These observations go beyond intuitive understanding.

### Step 2: Generate Derived Observations
Use the initial observations to brainstorm new, additional observations derived from them. This creates a richer understanding of the problem space.

### Step 3: Generate Natural Language Solution
Synthesize a natural language solution to the problem using all observations. The solution quotes relevant observations before each step.

### Step 4: Implement in Code
Convert the natural language solution into working code.

## How It Works in Practice

### Example: Competitive Programming Problem

**Original Problem:**
```
Given an array of integers, find the maximum sum of a contiguous subarray.
```

**Step 1 - Initial Observations:**
1. "A subarray with all negative numbers will have maximum sum equal to its single largest (least negative) element"
2. "The maximum sum could be the entire array if all elements are positive"
3. "We need to consider both positive and negative elements strategically"

**Step 2 - Derived Observations:**
1. "Since we must maintain contiguity, we can't skip elements. We must decide whether to include or exclude contiguous segments"
2. "Dynamic programming can solve this by tracking the maximum sum ending at each position"

**Step 3 - Natural Language Solution:**
"Use Kadane's algorithm: maintain the maximum sum ending at the current position. For each element, decide whether to extend the previous subarray or start a new one. Quote: 'The maximum sum could be the entire array...' - if all elements are positive, take all. Quote: '...we can't skip elements...' - so we track running sums."

**Step 4 - Implementation:**
```rust
fn max_subarray_sum(arr: &[i32]) -> i32 {
    let mut max_so_far = arr[0];
    let mut max_ending_here = arr[0];

    for &num in &arr[1..] {
        max_ending_here = std::cmp::max(num, max_ending_here + num);
        max_so_far = std::cmp::max(max_so_far, max_ending_here);
    }
    max_so_far
}
```

## When to Use PlanSearch

✅ **Best For:**
- Problem-solving tasks with clear structure
- Competitive programming challenges
- Algorithm design problems
- Tasks requiring step-by-step planning
- Domains where observations guide solution
- Code generation from problem specifications

❌ **Not Ideal For:**
- Creative writing or generation
- Open-ended opinion tasks
- Tasks requiring real-time response
- Very simple problems (overhead not justified)
- Domains with ambiguous solutions
- Tasks requiring only code (no planning needed)

## Configuration

```rust
use optillm_mars::strategies::plansearch::{PlanSearchConfig, PlanSearchAggregator};

// Default configuration
let config = PlanSearchConfig::new();

// Custom configuration
let config = PlanSearchConfig::new()
    .with_observation_temperature(0.8)
    .with_solution_temperature(0.7)
    .with_num_initial_observations(4)
    .with_num_derived_observations(3);

// Manual configuration
let config = PlanSearchConfig {
    observation_temperature: 0.7,      // High for diverse observations
    solution_temperature: 0.7,         // High for varied solutions
    implementation_temperature: 0.1,   // Low for correct implementation
    num_initial_observations: 3,
    num_derived_observations: 2,
    max_tokens_observations: 2048,
    max_tokens_solution: 4096,
    max_tokens_implementation: 4096,
};
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `observation_temperature` | 0.7 | Temperature for observation generation (should be high for diversity) |
| `solution_temperature` | 0.7 | Temperature for solution generation (high for varied approaches) |
| `implementation_temperature` | 0.1 | Temperature for code implementation (low for correctness) |
| `num_initial_observations` | 3 | Number of initial observations to generate |
| `num_derived_observations` | 2 | Number of observations derived from initial ones |
| `max_tokens_observations` | 2048 | Max tokens for each observation generation call |
| `max_tokens_solution` | 4096 | Max tokens for solution generation |
| `max_tokens_implementation` | 4096 | Max tokens for code implementation |

## Usage Example

```rust
use optillm_mars::strategies::plansearch::{PlanSearchConfig, PlanSearchAggregator};
use optillm_core::ModelClient;

// Configure PlanSearch
let config = PlanSearchConfig::new()
    .with_num_initial_observations(4)
    .with_observation_temperature(0.8);

// Run PlanSearch strategy
let result = PlanSearchAggregator::run_plansearch(
    "Given a list of numbers, implement quicksort",
    "You are an expert in algorithm design and implementation",
    config,
    &client
).await?;

println!("Natural Language Solution:\n{}", result.natural_language_solution);
println!("Code Implementation:\n{}", result.code_implementation);
println!("Observations: {}", result.metadata.observations_count);
println!("Tokens used: {}", result.metadata.total_tokens);
```

## Performance Characteristics

### Token Usage
Per execution (with default config):
- Initial observations: ~1,500 tokens
- Derived observations: ~1,500 tokens
- Natural language solution: ~2,500 tokens
- Code implementation: ~2,500 tokens
- **Total: ~8,000 tokens**

### Latency
- Sequential execution: 4 × generation_latency
- Typical latency: ~12-25 seconds with standard LLM

### Quality Impact
- **Best For:** Structured problem-solving
- **Improvement:** +20-30% on reasoning tasks with planning benefit
- **Advantage:** Produces both explanation and code

## Key Insights

### Observation-Guided Problem Solving
By explicitly generating observations, PlanSearch guides the model to think critically about the problem before solving it. This leads to better solutions.

### Temperature Strategy
- **Observation:** High (0.7) for diverse insights
- **Solution:** High (0.7) for varied approaches
- **Implementation:** Low (0.1) for correct, deterministic code

### Multi-Stage Refinement
Each stage builds on previous ones:
- Observations → Solution → Implementation
- Information flows forward, improving each stage

### Code Extraction
Automatically extracts code from markdown blocks with language tags, handling various formats gracefully.

## Advanced Features

### Observation Derivation
Derived observations leverage initial insights to create more sophisticated understanding. This two-stage approach reduces redundancy.

### Quoted Reasoning
Natural language solutions explicitly quote observations before each step, creating traceable, verifiable solutions.

### Format-Flexible Code Extraction
Handles code in various markdown formats:
- ` ```python ... ``` `
- ` ```rust ... ``` `
- ` ``` ... ``` ` (without language)
- Plain text (returned as-is)

## Comparison with Other Strategies

| Strategy | Approach | Cost | Best For |
|----------|----------|------|-------------|
| **Best-of-N** | Multiple attempts | Medium | Speed |
| **Self-Consistency** | Voting | High | Consensus |
| **PlanSearch** | Observation-guided | High | Problem-solving |
| **CoT Reflection** | Self-reflection | Medium | Reasoning |
| **LEAP** | Error learning | High | Pattern recognition |
| **RTO** | Round-trip | High | Code generation |

## Implementation Notes

- **Line-based parsing:** Observations parsed as individual non-empty lines
- **Code extraction:** Regex-based extraction from markdown code blocks
- **Observation count:** Total count includes both initial and derived observations
- **Fallback behavior:** Returns plain text if no markdown code blocks found
- **Token tracking:** Sum of all generation calls

## Common Pitfalls

⚠️ **Low observation temperature** - Observations become obvious and less helpful
⚠️ **Too many observations** - Overwhelming context, diminishing returns
⚠️ **High implementation temperature** - Generated code may be incorrect
⚠️ **Poor problem statement** - Unclear problems lead to poor observations
⚠️ **Task mismatch** - Works poorly on tasks not requiring planning
⚠️ **Missing language tags** - Code extraction may be imprecise without language markers

## References

**Related Concepts:**
- Tree of Thought reasoning
- Chain-of-Thought prompting
- Macro and micro reasoning
- Hierarchical problem decomposition
- Insight generation and synthesis

**Original Implementation:**
- Python optillm: `optillm/plansearch.py`
- Applicable to competitive programming, algorithm design, and structured problem-solving

## Future Enhancements

- **Multi-attempt Solutions:** Generate multiple solutions and select the best
- **Observation Ranking:** Rank observations by utility/relevance
- **Iterative Refinement:** Refine code based on compilation/test feedback
- **Constraint Tracking:** Explicitly track problem constraints in observations
- **Test Generation:** Automatically generate test cases from problem
- **Complexity Analysis:** Include complexity analysis in observations

