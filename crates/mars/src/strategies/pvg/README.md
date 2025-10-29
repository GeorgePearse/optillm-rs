# Prover-Verifier Game (PVG)

The Prover-Verifier Game is an adversarial optimization strategy that improves solution quality by generating both correct and intentionally flawed solutions, then using verification to distinguish between them.

## Algorithm Overview

PVG works through multiple rounds of adversarial generation and verification:

### Step 1: Helpful Solution Generation
Generate solutions intended to be correct, detailed, and well-reasoned.

### Step 2: Sneaky Solution Generation
Generate solutions that appear correct but contain subtle errors or flaws, designed to be plausible yet incorrect.

### Step 3: Unified Verification
Verify all solutions (both helpful and sneaky) using a separate verifier that scores each on correctness, clarity, and completeness.

### Step 4: Selection
Select the solution with the highest verification score.

### Step 5: Iterative Refinement (Optional)
For subsequent rounds:
- Refine the original query based on the best solution found
- Generate new helpful and sneaky solutions with the refined query
- Continue until the desired number of rounds are complete

## How It Works in Practice

### Example: Mathematical Problem

**Original Query:** "What is 12 × 8?"

**Helpful Solutions (Correct):**
- "12 × 8 = 96. We can think of this as 12 groups of 8."
- "Using multiplication: 12 × 8 = (10 + 2) × 8 = 80 + 16 = 96"
- "12 × 8 = 8 × 12 = 96 by commutativity"

**Sneaky Solutions (Flawed):**
- "12 × 8 = 106. When we multiply 12 by 8, we add 10 to get 106." (Incorrect)
- "12 × 8 = 92. We can compute this by multiplying 12 × 7 and adding 12, giving 92." (Math error)
- "12 × 8 = 94 because 12 × 8 is similar to 12 × 7 which is 84, plus 10 is 94." (Flawed reasoning)

**Verification:** Each solution gets a score based on correctness. Correct solution wins.

## When to Use PVG

✅ **Best For:**
- Problems with verifiable correct answers
- Mathematical or logical reasoning tasks
- Complex problem-solving where multiple approaches exist
- Situations requiring critical analysis of solutions
- Tasks where subtle errors are likely

❌ **Not Ideal For:**
- Creative or open-ended tasks with no single answer
- Problems where verification is ambiguous
- Tasks requiring rapid inference (multiple verification calls)
- Domains where "plausible-sounding" errors are hard to generate
- Real-time applications (expensive due to verification overhead)

## Configuration

```rust
use optillm_mars::strategies::pvg::{PVGConfig, PVGAggregator};

// Default configuration
let config = PVGConfig::new();

// Custom configuration
let config = PVGConfig::new()
    .with_num_solutions(5)
    .with_num_rounds(3)
    .with_initial_temperature(0.8);

// Manual configuration
let config = PVGConfig {
    num_solutions: 3,           // Solutions per mode
    num_rounds: 2,
    initial_temperature: 0.7,
    verification_temperature: 0.2,  // Low for objective evaluation
    refinement_temperature: 0.5,
    max_tokens_generation: 4096,
    max_tokens_verification: 1024,
    max_tokens_refinement: 1024,
};
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `num_solutions` | 3 | Solutions to generate per mode (helpful + sneaky) |
| `num_rounds` | 2 | Number of iteration rounds |
| `initial_temperature` | 0.7 | Temperature for generation (decreases per round) |
| `verification_temperature` | 0.2 | Temperature for verification (should be low) |
| `refinement_temperature` | 0.5 | Temperature for query refinement |
| `max_tokens_generation` | 4096 | Max tokens per solution |
| `max_tokens_verification` | 1024 | Max tokens for verification scoring |
| `max_tokens_refinement` | 1024 | Max tokens for query refinement |

## Usage Example

```rust
use optillm_mars::strategies::pvg::{PVGConfig, PVGAggregator};
use optillm_core::ModelClient;

// Configure PVG with 4 solutions per mode, 2 rounds
let config = PVGConfig::new()
    .with_num_solutions(4)
    .with_num_rounds(2);

// Run PVG strategy
let (solution, metadata) = PVGAggregator::run_pvg(
    "Prove that the sum of two even numbers is even",
    "You are a mathematics expert",
    config,
    &client
).await?;

println!("Best solution: {}", solution.answer);
println!("Verification score: {}", metadata.best_score);
println!("Total tokens: {}", metadata.total_tokens);
```

## Performance Characteristics

### Token Usage
Per round (with default config):
- Helpful solutions: 3 × 4096 tokens
- Sneaky solutions: 3 × 4096 tokens
- Verification (6 solutions): 6 × 1024 tokens
- Query refinement: 1 × 1024 tokens
- **Per round: ~29,000 tokens**
- **Two rounds: ~58,000 tokens**

### Latency
- Sequential generation: 2 × generation_latency per solution
- Verification: 6 × verification_latency
- Total: ~2-3 minutes with typical LLM latencies

### Quality Impact
- **Best For:** Logical/mathematical tasks
- **Improvement:** +15-25% on reasoning tasks
- **Advantage:** Catches subtle errors that other strategies miss

## Key Insights

### Adversarial Learning
PVG leverages adversarial thinking—generating intentionally incorrect solutions forces the model to think about what makes a solution wrong, which improves reasoning quality.

### Verification Quality
The quality of verification is critical. A good verifier needs to:
- Understand the domain
- Catch subtle errors
- Distinguish between plausible but wrong and correct solutions

### Temperature Strategy
- **Generation:** Moderate to high temperature (0.7) for diversity
- **Verification:** Low temperature (0.2) for objective, consistent evaluation
- **Refinement:** Medium temperature (0.5) for balanced query improvement

### Escalating Difficulty
Temperature decreases across rounds (0.7 → 0.6 → 0.5...), encouraging convergence to better solutions.

## Advanced Features

### Adversarial Prompting
The "sneaky mode" instruction encourages the model to think like an adversary, generating plausible-sounding incorrect solutions. This forces both:
1. The generation of diverse approaches
2. Critical analysis during verification

### Iterative Refinement
Query refinement between rounds allows the strategy to focus on problem aspects that weren't fully addressed:
- "Based on the best solution, what aspects were missing?"
- "What ambiguities in the query led to suboptimal solutions?"

### Score Extraction
Flexible score extraction handles various response formats:
- "Score: 8.5" (labeled)
- "8.5" (plain number)
- "8 out of 10" (ratio format)
- Clamping ensures 0-10 range

## Comparison with Other Strategies

| Strategy | Approach | Cost | Best For |
|----------|----------|------|----------|
| **Best-of-N** | Multiple attempts | Medium | Speed |
| **Self-Consistency** | Voting | High | Consensus |
| **PVG** | Adversarial game | Very High | Critical reasoning |
| **RTO** | Round-trip | High | Code generation |
| **MCTS** | Tree search | High | Planning |

## Implementation Notes

- **Adversarial pairs:** Always generates equal numbers of helpful and sneaky solutions
- **Verification independence:** Verifier doesn't know which mode generated each solution
- **Score normalization:** All scores clamped to 0-10 range
- **Round scaling:** Temperature decreases to encourage convergence
- **Flexible verification:** Handles various response formats gracefully

## Common Pitfalls

⚠️ **Weak verifier** - If the verifier can't distinguish correct from incorrect, PVG fails
⚠️ **Realistic sneaky solutions** - Need to actually be plausible or they don't add value
⚠️ **High verification temperature** - Will give inconsistent scores across identical solutions
⚠️ **Too many rounds** - Diminishing returns and exponential token cost
⚠️ **Unsuitable domains** - Creative tasks don't have "sneaky" solutions
⚠️ **Ambiguous queries** - Hard to generate meaningful "sneaky" solutions
⚠️ **High initial temperature** - Diverges from original query intent

## References

**Related Concepts:**
- Adversarial examples in machine learning
- Debate between agents for truth-seeking
- Self-play in game theory
- Verification and validation in formal methods

**Original Implementation:**
- Python optillm: `optillm/pvg.py`
- Applicable to reasoning, mathematics, logic, and verification tasks

## Future Enhancements

- **Adaptive verification:** Adjust verifier temperature based on solution disagreement
- **Adversarial weighting:** Weight solutions based on how "sneaky" they were
- **Query diversity:** Generate multiple refined queries instead of single refinement
- **Expert verification:** Use domain-specific verifiers instead of generic ones
- **Confidence scoring:** Combine verification score with confidence measures
