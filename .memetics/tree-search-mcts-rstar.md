---
upstream:
  - repo: codelion/optillm
    paths:
      - optillm/mcts.py
      - optillm/rstar.py
    ref: main
    last_seen: 5ea4efc217d24d89e8e352631c601553ea492900
local:
  paths:
    - crates/mars/src/strategies/mcts/mod.rs
    - crates/mars/src/strategies/r_star/mod.rs
---

# Tree search over dialogue states: MCTS and rStar

Rust port of optillm's `mcts.py` (Monte Carlo Tree Search over `DialogueState`s: select by UCB, expand by
asking the model for N candidate replies, roll out to depth D with simulated user turns, back-propagate
model-judged values) and a lightweight take on `rstar.py` (rStar: MCTS with richer, learned value
estimates and multiple action types). The MCTS port keeps upstream's `DialogueState` / `MCTSNode` / `MCTS`
structure and its "what might the user ask next" rollout prompt; it was reimplemented because the Rust
crate needs an arena-indexed tree (`Option<usize>` parents) and async streaming through
`optillm_core::ModelClient` instead of the OpenAI client and recursive Python objects.

Adaptation notes:

- `MCTSConfig { num_simulations, exploration_weight, simulation_depth, num_actions, ... }` mirrors the
  keyword args of upstream `chat_with_mcts`; keep defaults aligned with upstream when they change.
- Nodes are stored in a `Vec` and referenced by index; there are no parent back-pointers to Python
  objects. Any upstream algorithmic change (UCB formula, how `value` is parsed from the evaluation
  prompt, terminal condition) should be ported, structural refactors should not.
- `r_star/mod.rs` is a deliberately small (~150 line) approximation: UCB-driven candidate exploration with
  `num_simulations`, `exploration_constant`, `num_candidates`. It does not implement upstream rStar's
  full action set (A1-A5), code execution or answer-consistency verification, and it is not intended to
  until there is a Rust sandbox. Treat upstream `rstar.py` changes as "is the simplified version still
  faithful in spirit" rather than line-for-line.
