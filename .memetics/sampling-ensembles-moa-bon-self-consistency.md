---
upstream:
  - repo: codelion/optillm
    paths:
      - optillm/moa.py
      - optillm/bon.py
      - optillm/self_consistency.py
    ref: main
    last_seen: dabd59dced64d999ce6f86b2dd8cfc9cd537d1f8
local:
  paths:
    - crates/mars/src/strategies/moa/mod.rs
    - crates/mars/src/strategies/best_of_n/mod.rs
    - crates/mars/src/strategies/self_consistency/mod.rs
    - crates/mars/src/strategies/diverse_sampling/mod.rs
---

# Sample-and-select ensembles: MoA, Best-of-N, Self-Consistency

Rust ports of the three optillm "generate several completions, then pick or merge" techniques:
Mixture of Agents (`moa.py`: 3 high-temperature completions -> critique each -> synthesise a final answer),
Best-of-N (`bon.py`: N completions, rate them, return the best) and Self-Consistency
(`self_consistency.py`: K reasoning paths, extract answers, majority vote). `diverse_sampling` is a local
variant of Best-of-N that sweeps a temperature range instead of repeating one temperature. They were
ported rather than called because the Rust crate drives everything through the streaming
`optillm_core::ModelClient` trait and needs token accounting per phase.

Adaptation notes:

- Each strategy exposes a `*Config` struct plus an async `run`/`aggregate` that returns `Solution`s and
  per-phase token counts (e.g. `MoaMetadata`). Keep that shape; do not mirror upstream's
  `(text, completion_tokens)` tuple returns.
- MoA keeps upstream's three phases and prompt wording (critique prompt, synthesis prompt). If upstream
  changes the number of initial completions or the critique format, update `moa/mod.rs` to match.
- Best-of-N: upstream's single rating call is replaced by a `SelectionMethod` enum
  (verification score, confidence heuristic, reasoning length, conciseness) so selection is pluggable.
  The N/temperature defaults track upstream.
- Self-Consistency: answer extraction (`AnswerExtractionStrategy`) and voting (`VotingStrategy`,
  weighted or plain majority, `consensus_threshold`) are explicit enums rather than upstream's inline
  string matching; the majority-vote semantics should stay equivalent.
- `diverse_sampling` has no upstream file; it only shares the Best-of-N skeleton.
