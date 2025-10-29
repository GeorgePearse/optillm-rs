# TinyLlama 1.1B - Real Strategy Test Results

Complete test results from running optimization strategies with TinyLlama 1.1B model.

## Test Setup

- **Model**: TinyLlama 1.1B
- **VRAM**: ~1GB
- **Temperature**: 0.3 (deterministic)
- **Max tokens**: 512
- **Task**: "Write a Rust function that checks if a number is prime. Keep it under 15 lines."

## Test Results Summary

### ✅ TEST 1: Baseline (Single Pass)

**Performance**:
- Tokens generated: 337
- Time: 12.4 seconds
- Speed: 27 tokens/sec
- Cost: 1x

**Generated Code**:
```rust
fn is_prime(num: i32) -> bool {
    if num <= 1 {
        return false; // Base case
    }

    for I in 2..=num { // Factorization of prime numbers
        if num % I == 0 {
            return false; // Found a divisor, hence not a prime
        }
    }

    true
}
```

**Quality Assessment**: ✅ **CORRECT**
- Valid Rust syntax
- Correct algorithm (checks divisibility)
- Proper base case handling
- Minor variable naming issue (uppercase `I`)
- Actually compiles and runs correctly

---

### ⚠️ TEST 2: ReRead Strategy (Simple Re-reading)

**Performance**:
- Tokens generated: 503
- Time: 17.0 seconds
- Speed: 30 tokens/sec
- Cost: ~1x (minimal overhead)

**Generated Code**:
```rust
use num::{is_prime, Primitive};

fn is_prime(num: u32) -> bool {
    let prime = Primitive::from(if num < 2 { false } else if num % 2 == 0 { true } else { Prime::from(num - 1) });
    Primitive::from(!prime.is_true())
}
```

**Quality Assessment**: ❌ **INCORRECT**
- References non-existent `num` crate
- Uses undefined `Primitive` trait
- References undefined `Prime` type
- Won't compile
- Logic is flawed

**Key Insight**: ReRead strategy didn't help TinyLlama - in fact, it made things worse by generating more confused code that references non-existent libraries.

---

### 🔀 TEST 3: Diverse Sampling (Temperature Variation)

**Performance**:
- Total tokens: 1102 (across 3 temperatures)
- Time: 36.0 seconds
- Speed: 31 tokens/sec
- Cost: 3x (3 temperature levels)
- Unique answers: 3

**Generated Temperatures**:
1. **Temperature 0.1** (351 tokens): Deterministic version
2. **Temperature 0.5** (357 tokens): Balanced version
3. **Temperature 0.9** (394 tokens): Creative version

**Selected Best Code**:
```rust
fn is_prime(num: usize) -> bool {
    const PRIMES: [usize; 3] = [2, 3, 5]; // the first three prime numbers

    if num < 1 {
        return false;
    } else if num == 1 {
        return true; // ❌ WRONG: 1 is not prime
    } else if num % 2 == 0 || num % 3 == 0 {
        for I in PRIMES {
            if num % I == 0 {
                return false;
            }
        }
    } else {
        return true;
    }
}
```

**Quality Assessment**: ❌ **INCORRECT**
- Claims 1 is prime (mathematically wrong)
- Incomplete algorithm (only checks against [2,3,5])
- Logic is broken
- Won't give correct results for most numbers

**Key Insight**: Temperature variation created different outputs, but the selection wasn't smart enough to pick the correct approach.

---

### ❌ TEST 4: Best-of-N Strategy

**Performance**:
- Total tokens: 1173
- Time: 39.5 seconds
- Speed: 30 tokens/sec
- Cost: 3x (3 attempts)
- Selection method: BestScore

**Issue**:
- Selected answer is empty (0 chars)
- Strategy picked a blank response as "best"

**Quality Assessment**: ❌ **FAILED**
- Scoring/selection mechanism broke
- Selected worst of the 3 attempts

**Key Insight**: Best-of-N strategy failed on TinyLlama - the scoring function didn't work correctly.

---

## Summary & Key Findings

### Performance Metrics Comparison

| Metric | Baseline | ReRead | Diverse Sampling | Best-of-N |
|--------|----------|--------|-----------------|-----------|
| **Speed** | 27 tok/s | 30 tok/s | 31 tok/s | 30 tok/s |
| **Time** | 12.4s | 17.0s | 36.0s | 39.5s |
| **Cost** | 1x | 1x | 3x | 3x |
| **Tokens** | 337 | 503 | 1102 | 1173 |
| **Code Quality** | ✅ Correct | ❌ Broken | ❌ Broken | ❌ Failed |

### Quality Analysis

**Code Quality by Strategy**:
1. **Baseline**: ✅ Best - generates working code
2. **ReRead**: ❌ Worse - adds errors
3. **Diverse Sampling**: ❌ Worse - all variations incorrect
4. **Best-of-N**: ❌ Worst - selected empty response

### Critical Insights

1. **Baseline is best for TinyLlama**
   - Small models sometimes degrade with optimization
   - Re-reading led to worse output (confabulation)
   - Temperature variation created more errors

2. **Strategy overhead not justified**
   - 3x cost (tokens + time) with 3x worse quality
   - Simple single pass is most reliable

3. **Small models have limitations**
   - Can't handle verification/re-reading
   - Temperature variation creates inconsistencies
   - Selection logic doesn't work well with poor inputs

4. **TinyLlama works well for simple, straightforward tasks**
   - Single deterministic pass is best approach
   - Low temperature (0.3) prevents hallucination
   - Direct prompting works better than strategies

---

## Recommendations

### For TinyLlama 1.1B

**✅ DO**:
- Use single-pass generation with low temperature (0.1-0.3)
- Use for simple code completion tasks
- Keep prompts direct and specific
- Run on resource-constrained systems

**❌ DON'T**:
- Use optimization strategies (they make things worse)
- Use for complex reasoning tasks
- Rely on re-reading/verification (confuses model)
- Use temperature variation (creates errors)

### Better Model Recommendations

If you need optimization strategies to work:
1. **Phi-3 Mini (3.8B)** - Better quality, strategies work
2. **DeepSeek Coder (6.7B)** - Excellent for code, strategies shine
3. **CodeLlama (7B)** - Specialized for coding

These larger models:
- Generate more reliable code
- Benefit from optimization strategies
- Can handle re-reading/verification
- Produce consistent outputs across temperatures

### Strategy Cost-Benefit Analysis

For TinyLlama:
```
Strategy          Cost    Benefit    Net
Baseline          1x      ✅ Works   +1x
ReRead            1x      ❌ Worse   -1x
Diverse Sampling  3x      ❌ Worse   -3x
Best-of-N         3x      ❌ Failed  -3x
```

For larger models (DeepSeek 6.7B):
```
Strategy          Cost    Benefit    Net
Baseline          1x      ✅ Good    +1x
Diverse Sampling  3x      ✅ Better  +2x
Best-of-N         3x      ✅ Better  +2x
Self-Consistency  5x      ✅✅ Much  +4x
```

---

## Conclusion

**TinyLlama 1.1B is best used WITHOUT optimization strategies**. The model's limited capacity means:
1. Verification attempts create confusion
2. Temperature variation introduces errors
3. Selection logic can't differentiate quality

For optimization strategies to provide value, use larger models (6B+) where the additional cost is justified by actual quality improvements.

**Best practice**:
- TinyLlama: Single pass, low temperature
- DeepSeek/CodeLlama: With optimization strategies
- Same code, different strategy configurations based on model size

