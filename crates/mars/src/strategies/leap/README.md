# LEAP (Learning from Errors for Adaptive Process)

LEAP is a strategy that extracts few-shot examples from queries, learns from intentional mistakes, and applies derived principles to improve answer quality. It's particularly effective for tasks where examples demonstrate patterns that can be learned.

## Algorithm Overview

LEAP works through five main steps:

### Step 1: Example Extraction
Analyze the query to identify and extract any few-shot examples embedded within it. Examples are formatted as question-answer pairs.

### Step 2: Mistake Generation
For each extracted example, deliberately generate an incorrect answer. This forces the model to think about what makes a solution wrong.

### Step 3: Low-Level Principle Generation
Analyze each mistake by comparing it to the correct answer. Extract specific insights about why the mistake occurred and what principles would have prevented it.

### Step 4: High-Level Principle Consolidation
Combine and consolidate low-level principles into fewer, more general high-level principles that capture the essence of the learning.

### Step 5: Final Answer Generation
Use the learned principles as guidance to answer the original query with improved accuracy.

## How It Works in Practice

### Example: Mathematical Problem

**Original Query:**
```
Solve these problems:
Example 1: 5 + 3 = 8
Example 2: 12 × 4 = 48
Now solve: 7 × 6 = ?
```

**Step 1 - Examples Extracted:**
- Question: "5 + 3 = ?", Answer: "8"
- Question: "12 × 4 = ?", Answer: "48"

**Step 2 - Mistakes Generated:**
- For "12 × 4": Generated "52" instead of "48"
  - Reasoning: "12 × 4, I'll multiply 12 by 5 and subtract 8... that gives 52"

**Step 3 - Low-Level Principles:**
- "Don't modify the problem operands - use them exactly as given"
- "Verify intermediate calculations before final answer"
- "Follow standard order of operations precisely"

**Step 4 - High-Level Principles:**
- "Always use the exact operands provided without modification"
- "Verify each calculation step independently"

**Step 5 - Final Answer:**
Using the learned principles: "7 × 6 = 42"

## When to Use LEAP

✅ **Best For:**
- Tasks with few-shot examples in the prompt
- Problems where patterns can be learned from examples
- Domain-specific reasoning where principles apply broadly
- Educational contexts with worked examples
- Mathematical or logical reasoning with clear patterns

❌ **Not Ideal For:**
- Queries without embedded examples
- Creative or open-ended tasks with no clear rules
- Tasks requiring real-time response (extraction adds latency)
- Domains where mistakes are hard to generate plausibly
- Very simple queries that don't benefit from principle learning

## Configuration

```rust
use optillm_mars::strategies::leap::{LEAPConfig, LEAPAggregator};

// Default configuration
let config = LEAPConfig::new();

// Custom configuration
let config = LEAPConfig::new()
    .with_extraction_temperature(0.2)
    .with_mistake_temperature(0.8)
    .with_max_principles(6);

// Manual configuration
let config = LEAPConfig {
    extraction_temperature: 0.3,      // Low for precise extraction
    mistake_temperature: 0.7,          // High for diverse mistakes
    principle_temperature: 0.3,        // Low for clarity
    final_temperature: 0.5,            // Balanced
    max_tokens_extraction: 2048,
    max_tokens_mistakes: 2048,
    max_tokens_principles: 2048,
    max_tokens_final: 2048,
    max_principles: 8,
};
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `extraction_temperature` | 0.3 | Temperature for identifying and extracting examples |
| `mistake_temperature` | 0.7 | Temperature for mistake generation (should be high) |
| `principle_temperature` | 0.3 | Temperature for principle generation (should be low) |
| `final_temperature` | 0.5 | Temperature for final answer generation |
| `max_tokens_extraction` | 2048 | Max tokens for example extraction |
| `max_tokens_mistakes` | 2048 | Max tokens for mistake generation per example |
| `max_tokens_principles` | 2048 | Max tokens for principle generation |
| `max_tokens_final` | 2048 | Max tokens for final answer |
| `max_principles` | 8 | Maximum number of high-level principles to keep |

## Usage Example

```rust
use optillm_mars::strategies::leap::{LEAPConfig, LEAPAggregator};
use optillm_core::ModelClient;

// Configure LEAP with custom parameters
let config = LEAPConfig::new()
    .with_extraction_temperature(0.2)
    .with_max_principles(6);

// Run LEAP strategy
let result = LEAPAggregator::run_leap(
    "Given examples: A→1, B→2. What is C→?",
    "You are a logical reasoning expert",
    config,
    &client
).await?;

println!("Answer: {}", result.answer);
println!("Examples extracted: {}", result.metadata.examples_extracted);
println!("Principles learned: {}", result.metadata.principles_learned);
println!("Final principles: {:?}", result.metadata.final_principles);
println!("Total tokens: {}", result.metadata.total_tokens);
```

## Performance Characteristics

### Token Usage
Per execution (with default config):
- Example extraction: ~1,500 tokens
- Mistakes (3 examples): 3 × 2,000 = ~6,000 tokens
- Low-level principles: 3 × 1,500 = ~4,500 tokens
- High-level principles: ~1,500 tokens
- Final answer: ~1,500 tokens
- **Total: ~15,000 tokens**

### Latency
- Sequential generation: 5 × generation_latency
- Typical latency: ~15-30 seconds with standard LLM

### Quality Impact
- **Best For:** Tasks with clear example patterns
- **Improvement:** +10-20% on tasks with learnable patterns
- **Advantage:** Generalizable principles for broader applicability

## Key Insights

### Learning from Errors
LEAP leverages the principle that understanding what makes something wrong is key to getting it right. By deliberately generating mistakes, the model develops a deeper understanding of correct reasoning.

### Example Extraction Quality
The quality of extracted examples determines the quality of learned principles. Low-quality extraction leads to irrelevant principles.

### Temperature Strategy
- **Extraction:** Low (0.3) for precise, consistent example identification
- **Mistakes:** High (0.7) for diverse, realistic errors
- **Principles:** Low (0.3) for clear, precise insights
- **Final:** Medium (0.5) for balanced application

### Principle Consolidation
High-level principles are more generalizable than low-level ones. Consolidation reduces redundancy and focuses on core insights.

## Advanced Features

### Principle-Guided Generation
The final generation step explicitly includes learned principles in the prompt, directing the model to apply them consciously.

### Fallback Behavior
If no examples are found in the query, LEAP gracefully falls back to standard generation without the principle-learning phase.

### Multi-Stage Learning
The hierarchical approach (low-level → high-level) allows principles to be progressively refined and consolidated.

## Comparison with Other Strategies

| Strategy | Approach | Cost | Best For |
|----------|----------|------|------------|
| **Best-of-N** | Multiple attempts | Medium | Speed/diversity |
| **Self-Consistency** | Voting | High | Consensus |
| **PVG** | Adversarial game | Very High | Critical reasoning |
| **LEAP** | Example learning | High | Pattern recognition |
| **RTO** | Round-trip | High | Code generation |
| **CoT Reflection** | Self-reflection | Medium | Reasoning improvement |

## Implementation Notes

- **Example Format:** JSON array with "question" and "answer" fields
- **Output Extraction:** Uses regex to extract content between `<output>` tags
- **Mistake Validation:** Only counted if different from correct answer
- **Empty Example Fallback:** Proceeds with direct answer if no examples found
- **Token Estimation:** Rough approximation based on text length

## Common Pitfalls

⚠️ **No examples in query** - LEAP skips learning and provides direct answer
⚠️ **Poor example quality** - Garbage in, garbage out for principles
⚠️ **Too many principles** - Overwhelming and contradictory guidance
⚠️ **High extraction temperature** - Extracts non-existent examples
⚠️ **Low mistake temperature** - Generates only obvious, unhelpful mistakes
⚠️ **Complex principle extraction** - Failure to consolidate leads to noise
⚠️ **Task mismatch** - Works poorly on creative or judgment-based tasks

## References

**Related Concepts:**
- Few-shot learning in machine learning
- Exemplar-based reasoning
- Transfer learning from examples
- Error analysis in education
- Principle extraction from case studies

**Original Implementation:**
- Python optillm: `optillm/leap.py`
- Applicable to reasoning, pattern recognition, and learning-from-examples tasks

## Future Enhancements

- **Adaptive Extraction:** Automatically detect example format variations
- **Principle Weighting:** Weight principles by their extraction confidence
- **Cross-Example Learning:** Learn from relationships between examples
- **Iterative Refinement:** Multiple rounds of principle refinement
- **Domain-Specific Rules:** Custom principle formats for specific domains
- **Example Verification:** Verify that extracted examples are actually correct

