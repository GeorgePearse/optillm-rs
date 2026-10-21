---
upstream:
  - repo: codelion/optillm
    paths:
      - optillm/mars/mars.py
      - optillm/mars/agent.py
      - optillm/mars/aggregator.py
      - optillm/mars/verifier.py
      - optillm/mars/strategy_network.py
      - optillm/mars/workspace.py
      - optillm/mars/prompts.py
      - optillm/mars/answer_extraction.py
    ref: main
    last_seen: c0dc4f011ad644ce44aea884c43fbb0de6beb0e3
local:
  paths:
    - crates/mars/src/core/coordinator.rs
    - crates/mars/src/core/agent.rs
    - crates/mars/src/core/aggregator.rs
    - crates/mars/src/core/verifier.rs
    - crates/mars/src/core/strategy.rs
    - crates/mars/src/core/workspace.rs
    - crates/mars/src/core/prompts.rs
    - crates/mars/src/config.rs
    - crates/mars/src/strategies/rsa/mod.rs
---

# MARS multi-agent reasoning system

Rust port of optillm's MARS (Multi-Agent Reasoning System): N agents with diverse temperatures explore
a problem in parallel, their solutions land in a shared workspace, an optional RSA-inspired aggregator and
a cross-agent strategy network refine the population, a verifier scores each solution, unverified
solutions are iteratively improved, and a final synthesis step picks the answer (majority vote, best
verified, or synthesis from the top solutions). The upstream Python package is split one file per
concern (`mars.py` orchestrator, `agent.py`, `aggregator.py`, `verifier.py`, `strategy_network.py`,
`workspace.py`, `prompts.py`, `answer_extraction.py`) and the Rust crate mirrors that split under
`crates/mars/src/core/` (`coordinator.rs` is the `mars.py` orchestrator; `strategy.rs` is
`strategy_network.py`). It was re-implemented rather than wrapped because the target is an embedded Rust
runtime (the `code` Codex fork) that cannot carry a Python proxy, and because all LLM calls go through
the crate's own `optillm_core::ModelClient` streaming trait instead of an OpenAI client object.

Adaptation notes (should stay stable even if upstream changes):

- Public surface is `MarsCoordinator::run_with_client(query, &client)` returning `MarsOutput` with a
  `SelectionMethod` and a stream of `MarsEvent`s; `MarsConfig` (and its `with_advanced_features()`
  builder) is the single knob set. Upstream's `multi_agent_reasoning_system(...)` function signature and
  its `(response, tokens)` tuple are deliberately not mirrored.
- The five-phase structure (explore -> RSA aggregation / strategy network -> verify -> improve -> synthesise)
  and the prompt texts in `prompts.rs` track upstream `prompts.py`; keep the prompts in sync when upstream
  rewrites them, but keep `<think>` tag handling as an option, not mandatory.
- `verifier.rs` is currently a placeholder that returns a positive verification (TODO: wire the
  ModelClient). Any upstream change to the verifier's consensus rule (two consecutive CORRECT) or scoring
  format is the most important drift to pick up.
- `strategies/rsa/mod.rs` generalises the aggregator's RSA loop (population N, select K, T iterations,
  elitism) into a standalone strategy with pluggable `SelectionCriterion` / `RefinementStrategy`; upstream
  has no standalone RSA module, so only the aggregator semantics should be compared.
- Answer extraction is simplified: no separate `answer_extraction.py` equivalent, extraction lives in
  the coordinator's final synthesis.
