# MARS Verification

Solution verification is critical for ensuring answer quality in MARS.

## Overview

Verification involves cross-validating solutions across multiple dimensions:

- **Correctness**: Is the answer factually correct?
- **Reasoning**: Is the logic sound?
- **Completeness**: Does it address all aspects?
- **Clarity**: Is it well-explained?

## Verification Process

1. **Cross-Agent Verification**: Agents verify each other
2. **Scoring**: Solutions receive numerical scores (0.0-1.0)
3. **Thresholding**: Only high-scoring solutions proceed
4. **Feedback**: Low scores trigger improvement

## Verifier Component

```rust
pub struct Verifier {
    system_prompt: String,
}

impl Verifier {
    pub async fn verify_solution(
        &self,
        solution: &Solution,
        client: &dyn ModelClient,
    ) -> Result<f32> {
        // Generate verification prompt
        // Score the solution
        // Return score
    }
}
```

## Verification Score

Scores represent confidence in solution quality:

| Score | Interpretation |
|-------|-----------------|
| 0.9-1.0 | Excellent, ready to use |
| 0.7-0.9 | Good, minor improvements possible |
| 0.5-0.7 | Acceptable, needs refinement |
| 0.0-0.5 | Poor, requires major work |

## Configuration

Verification is configured via `MarsConfig`:

```rust
pub struct MarsConfig {
    pub num_verification_rounds: usize,
    pub verification_threshold: f32,
}
```

### Verification Rounds

Number of verification passes:

```rust
// Default: 2 rounds
config.num_verification_rounds = 3;  // More thorough
```

### Verification Threshold

Minimum score to accept solution:

```rust
// Default: 0.7
config.verification_threshold = 0.8;  // Stricter
```

## Multi-Round Verification

```
Round 1: Initial Verification
  ├─ Agent 0 verifies Agent 1 & 2
  ├─ Agent 1 verifies Agent 0 & 2
  └─ Agent 2 verifies Agent 0 & 1

Round 2: Secondary Verification
  └─ Check verified solutions again
```

## Score Aggregation

When multiple agents verify a solution:

```rust
let scores: Vec<f32> = agents.iter()
    .map(|a| a.verify(&solution, client).await?)
    .collect::<Result<_>>()?;

let avg_score = scores.iter().sum::<f32>() / scores.len() as f32;
let is_verified = avg_score >= threshold;
```

## Verification Failure Handling

If solution fails verification:

```rust
if solution.verification_score < threshold {
    // Trigger improvement phase
    improved = coordinator.improve_solution(
        solution,
        verification_feedback,
    ).await?;
}
```

## Custom Verification

Implement custom verification logic:

```rust
pub async fn custom_verify(
    solution: &Solution,
    domain_rules: &DomainRules,
) -> Result<f32> {
    // Check domain-specific constraints
    if !domain_rules.validate(&solution.answer)? {
        return Ok(0.0);
    }

    // Check format
    if !is_valid_format(&solution.answer)? {
        return Ok(0.2);
    }

    // Check completeness
    if !is_complete(&solution)? {
        return Ok(0.5);
    }

    Ok(0.9)
}
```

## Performance Impact

- **More rounds** = Better quality, slower
- **Stricter threshold** = Higher quality, more iterations
- **Fewer agents** = Faster, less thorough

See [MARS Overview](overview.md) and [Configuration](configuration.md).
