---
upstream:
  - repo: codelion/optillm
    paths:
      - optillm/cot_decoding.py
      - optillm/entropy_decoding.py
      - optillm/thinkdeeper.py
      - optillm/autothink/autothink.py
      - optillm/autothink/classifier.py
      - optillm/autothink/processor.py
      - optillm/autothink/steering.py
    ref: main
    last_seen: c8cbdde8fe53916231f3ab829868a9f762fd3ca0
local:
  paths:
    - crates/mars/src/strategies/cot_decoding/mod.rs
    - crates/mars/src/strategies/entropy_decoding/mod.rs
    - crates/mars/src/strategies/deep_thinking/mod.rs
    - crates/mars/src/strategies/autothink/mod.rs
---

# Prompt-level approximations of optillm's decoding-time techniques

optillm's `cot_decoding.py`, `entropy_decoding.py`, `thinkdeeper.py` and the `autothink/` package all
operate *inside* the decoding loop of a locally loaded HuggingFace model: CoT-Decoding inspects top-k
logits of the first token and scores paths by answer-token confidence, entropy decoding adapts sampling
from the attention/logit entropy, ThinkDeeper manipulates the `<think>` budget by injecting continuation
phrases, and AutoThink classifies query complexity and applies steering vectors during generation.
optillm-rs only talks to models through a chat-completions `ModelClient`, so each of these was
re-expressed as a prompt- and sampling-level approximation: `cot_decoding` asks for N explicit reasoning
steps and verifies them; `entropy_decoding` draws several samples and selects by a Shannon-entropy
diversity score; `deep_thinking` estimates difficulty and scales the token budget / iteration count;
`autothink` classifies complexity from token count and keywords and picks temperature and depth.

Adaptation notes:

- These are intentionally not faithful ports. When upstream changes, port the *policy* (e.g. new
  complexity thresholds, new think-budget heuristics, changed continuation phrases) not the logit-level
  mechanics, which cannot be reproduced over an API.
- Each strategy keeps a small `*Config` (`CotDecodingConfig { num_steps, enable_verification }`,
  `EntropyDecodingConfig { target_entropy, num_samples }`, `DeepThinkingConfig { min_tokens, max_tokens,
  num_iterations }`, `AutoThinkConfig` thresholds) as its API; keep these stable.
- AutoThink steering vectors (`steering.py`) have no local equivalent and should not be added without a
  local-model backend.
- If a local-model backend ever lands in the crate, these four modules are the candidates to be
  replaced with real decoding-time implementations following upstream.
