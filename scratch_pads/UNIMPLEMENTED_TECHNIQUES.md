# Unimplemented OptimLLM Techniques

This document tracks optimization techniques from the Python OptimLLM library that have not yet been implemented in the Rust version.

## Summary

- **Python OptimLLM**: 20+ optimization techniques
- **Rust optillm-rs**: 13 implemented techniques
- **Gap**: 7+ techniques identified as not yet ported

## Implemented Techniques (13)

| Technique | Status | Location |
|-----------|--------|----------|
| MARS (Multi-Agent Reasoning System) | ✅ Complete | `crates/mars/src/core/coordinator.rs` |
| MOA (Mixture of Agents) | ✅ Complete | `crates/mars/src/strategies/moa/` |
| Self-Consistency | ✅ Complete | `crates/mars/src/strategies/self_consistency/` |
| Best-of-N | ✅ Complete | `crates/mars/src/strategies/best_of_n/` |
| RSA (Reinforced Self-Aggregation) | ✅ Complete | `crates/mars/src/strategies/rsa/` |
| MCTS (Monte Carlo Tree Search) | ✅ Complete | `crates/mars/src/strategies/mcts/` |
| CoT Reflection | ✅ Complete | `crates/mars/src/strategies/cot_reflection/` |
| RTO (Round-Trip Optimization) | ✅ Complete | `crates/mars/src/strategies/rto/` |
| PVG (Prover-Verifier Game) | ✅ Complete | `crates/mars/src/strategies/pvg/` |
| LEAP (Learning from Errors) | ✅ Complete | `crates/mars/src/strategies/leap/` |
| PlanSearch | ✅ Complete | `crates/mars/src/strategies/plansearch/` |
| ReRead | ✅ Complete | `crates/mars/src/strategies/reread/` |
| Diverse Sampling | ✅ Complete | `crates/mars/src/strategies/diverse_sampling/` |

## Unimplemented Techniques

### 1. CePO (Cerebras Planning & Optimization)

**Priority**: 🔴 High
**Complexity**: High
**Status**: Not started

**Description**:
Advanced planning-based optimization that combines structured planning with verification. Focuses on decomposing complex problems into manageable steps with intermediate verification.

**Key Components**:
- Problem decomposition and planning phase
- Step-by-step execution with progress tracking
- Intermediate result verification
- Backtracking and re-planning on failures
- Integration with tree-search algorithms

**Implementation Notes**:
- Would integrate well with existing MCTS implementation
- Requires planner module (similar to PlanSearch but more sophisticated)
- Needs state tracking for problem decomposition
- Should support multi-step reasoning with verification at each stage

**Python Reference**: `optillm.strategies.cepo`

**Potential File Structure**:
```
crates/mars/src/strategies/cepo/
├── mod.rs
├── planner.rs
├── executor.rs
└── verifier.rs
```

---

### 2. AutoThink

**Priority**: 🟡 Medium
**Complexity**: Medium
**Status**: Not started

**Description**:
Query complexity classification with adaptive reasoning depth. Analyzes input query characteristics and automatically determines optimal reasoning depth (shallow, medium, deep).

**Key Components**:
- Query complexity classifier (token count, keyword analysis, structure analysis)
- Complexity scoring (0.0-1.0 scale)
- Adaptive depth selector
- Temperature and iteration adjustment based on complexity
- Steering vector support for inference-time guidance

**Implementation Notes**:
- Would add to core coordinator logic
- Requires simple ML-based classifier or heuristic-based scorer
- Could use regex patterns for keyword detection
- Steering vectors would need integration with provider routing
- Lightweight feature compared to other techniques

**Python Reference**: `optillm.strategies.autothink`

**Potential Changes**:
- New module: `crates/mars/src/strategies/autothink/`
- Extends `MarsConfig` with complexity thresholds
- Modifies agent selection logic based on complexity

---

### 3. LongCePO (Long Context CePO)

**Priority**: 🟡 Medium
**Complexity**: Very High
**Status**: Not started

**Description**:
Extends CePO for handling infinite or very long context by dividing problems into chunks and solving recursively. Implements divide-and-conquer strategy for context-window limited scenarios.

**Key Components**:
- Context window estimation
- Problem chunking strategy
- Recursive decomposition
- Sub-problem solver coordination
- Results merging and synthesis
- Cross-chunk dependency resolution

**Implementation Notes**:
- Would build on top of CePO implementation
- Requires context accounting and windowing logic
- Needs sophisticated merging strategy for sub-results
- Complex to test with actual long contexts
- Potentially lower priority than core techniques

**Python Reference**: `optillm.strategies.longcepo`

**Dependency**: Requires CePO to be implemented first

---

### 4. Z3 Solver Integration

**Priority**: 🔴 High
**Complexity**: Very High
**Status**: Not started

**Description**:
Integration with Z3 theorem prover for logical reasoning and constraint satisfaction. Enables formal verification and symbolic reasoning for problems with logical structure.

**Key Components**:
- Problem-to-Z3-syntax conversion
- Constraint extraction from reasoning
- Model generation and validation
- Solution verification via Z3
- Fallback to heuristic solving for non-formalizable problems

**Implementation Notes**:
- Would require `z3-sys` Rust bindings
- Complex integration - needs problem classification
- High barrier to entry but potentially high impact
- Best for logic puzzles, constraint satisfaction, formal verification tasks
- May require custom prompt engineering for Z3 output

**Python Reference**: `optillm.strategies.z3_solver`

**Potential Dependencies**:
- `z3-sys` crate (LLVM-based)
- Custom parsing for Z3 output

---

### 5. R* Algorithm

**Priority**: 🟡 Medium
**Complexity**: High
**Status**: Not started

**Description**:
Advanced tree search algorithm combining systematic exploration with rollout evaluation. Improves upon MCTS by using learned value estimates and more sophisticated node selection.

**Key Components**:
- Enhanced UCB formula with learned priors
- Value network for position evaluation
- Rollout policy (may use smaller model or heuristic)
- Extended search tree building
- Best-path extraction from search tree

**Implementation Notes**:
- Would enhance existing MCTS implementation
- Requires integration with model client for value estimation
- Rollout policy could use diverse sampling or temperature variation
- Significant upgrade over current MCTS but reuses core search structure

**Python Reference**: `optillm.strategies.r_star`

**Potential Changes**:
- Extends `crates/mars/src/strategies/mcts/`
- New module for value estimation
- Enhanced node selection logic

---

### 6. Deep Thinking / Deep Confidence

**Priority**: 🟡 Medium
**Complexity**: Medium
**Status**: Not started

**Description**:
Inference-time scaling technique that allocates more computation (tokens) to harder problems. Uses problem difficulty estimation to determine reasoning length and model confidence scoring.

**Key Components**:
- Problem difficulty estimator
- Confidence scoring mechanism
- Dynamic token allocation
- Repeated reasoning with increasing depth
- Uncertainty quantification

**Implementation Notes**:
- Similar to AutoThink but focuses on token allocation rather than strategy selection
- Could use answer consistency (across multiple runs) as confidence metric
- Useful for distinguishing easy vs hard problems
- Lightweight implementation possible

**Python Reference**: `optillm.strategies.deep_thinking`

**Potential File Structure**:
```
crates/mars/src/strategies/deep_thinking/
├── mod.rs
├── difficulty_estimator.rs
└── confidence_scorer.rs
```

---

### 7. CoT Decoding

**Priority**: 🟢 Low-Medium
**Complexity**: Medium
**Status**: Not started

**Description**:
Advanced chain-of-thought variant that guides decoding process to follow reasoning structure. Different from CoT Reflection in that it modifies the decoding/sampling process itself rather than post-hoc reflection.

**Key Components**:
- Reasoning prefix templates
- Constrained decoding (if provider supports)
- Structured output parsing
- Intermediate step verification
- Backtracking on invalid reasoning

**Implementation Notes**:
- Requires prompt engineering for effective reasoning structure
- May need provider-specific implementation (OpenAI has special support)
- Could integrate with PlanSearch or PVG for structured reasoning
- Less essential than planning-based techniques

**Python Reference**: `optillm.strategies.cot_decoding`

---

### 8. Entropy Decoding

**Priority**: 🟢 Low
**Complexity**: Low-Medium
**Status**: Not started

**Description**:
Entropy-based decoding that samples based on output entropy rather than just temperature. Provides more fine-grained control over diversity and quality tradeoff.

**Key Components**:
- Token-level entropy calculation
- Entropy-based sampling selector
- Diversity vs quality balance parameter
- Multiple sample collection and ranking

**Implementation Notes**:
- Simplest of the unimplemented techniques
- Could be added as alternative to temperature-based sampling
- Requires access to token probabilities from provider
- May not be supported by all LLM providers

**Python Reference**: `optillm.strategies.entropy_decoding`

---

## Implementation Priority Matrix

```
         Complexity
         Low  Medium  High
Impact ┌────┬────┬────┐
High   │ S  │ C  │ Z3 │
       ├────┼────┼────┤
Medium │ ED │ AT │ R* │
       │ CD │DT/DC    │
       ├────┼────┼────┤
Low    │    │ LS │    │
       └────┴────┴────┘

Legend:
S = Strategy framework
C = CePO
Z3 = Z3 Solver
ED = Entropy Decoding
AT = AutoThink
R* = R* Algorithm
CD = CoT Decoding
DT/DC = Deep Thinking/Deep Confidence
LS = LongCePO

Recommended Order:
1. AutoThink (medium impact, medium complexity)
2. R* Algorithm (medium-high impact, high complexity)
3. CePO (high impact, high complexity)
4. Deep Thinking (medium impact, medium complexity)
5. Z3 Solver (high impact, very high complexity)
6. CoT Decoding (low impact, medium complexity)
7. Entropy Decoding (low impact, low complexity)
8. LongCePO (medium impact, very high complexity) - depends on CePO
```

## Integration Considerations

### Coordinator Changes
Many techniques will require updates to `MarsCoordinator`:
- New configuration fields
- Extended event types
- Modified optimization loop

### Type System
May need new types in `crates/mars/src/types.rs`:
- Complexity classification
- Planning state
- Search tree structures
- Solver integration types

### Provider Requirements
- **Z3 Solver**: May require local solver setup or API integration
- **Steering Vectors**: Requires provider support (OpenAI GPT-4 with reasoning)
- **Token Probabilities**: Needed for entropy decoding

### Testing Strategy
Each new technique should include:
- Unit tests for core logic
- Integration tests with mock model client
- Benchmark comparisons against existing techniques
- Documentation with usage examples

## References

- **Python OptimLLM**: https://github.com/codelion/optillm
- **Research Papers**: Referenced in Python implementation docstrings
- **Benchmarks**: Available in `scratch_pads/COMPREHENSIVE_STRATEGY_BENCHMARK_RESULTS.md`

## Notes for Future Implementation

- Start with AutoThink for quick win and experience with complexity classification
- CePO is foundational - implement before LongCePO
- Z3 Solver is high-impact but requires significant infrastructure
- Consider community contributions for complex techniques
- Maintain backward compatibility with existing coordinator API
