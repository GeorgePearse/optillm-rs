---
upstream:
  - repo: codelion/optillm
    paths:
      - optillm/cot_reflection.py
      - optillm/reread.py
      - optillm/rto.py
      - optillm/leap.py
      - optillm/plansearch.py
      - optillm/pvg.py
    ref: main
    last_seen: fa1a71ae0074644018be7555e7b6a6f4fec47b71
local:
  paths:
    - crates/mars/src/strategies/cot_reflection/mod.rs
    - crates/mars/src/strategies/reread/mod.rs
    - crates/mars/src/strategies/rto/mod.rs
    - crates/mars/src/strategies/leap/mod.rs
    - crates/mars/src/strategies/plansearch/mod.rs
    - crates/mars/src/strategies/pvg/mod.rs
---

# Multi-step prompting pipelines: CoT-Reflection, RE2, RTO, LEAP, PlanSearch, PVG

Rust ports of optillm's prompt-orchestration techniques, each of which is a fixed sequence of model calls
with specific prompt templates and regex-based extraction:

- `cot_reflection.py` -> `cot_reflection/`: `<thinking>` / `<reflection>` / `<output>` tag prompt, extract
  the output section.
- `reread.py` -> `reread/`: RE2, the query is repeated with a re-read instruction before answering.
- `rto.py` -> `rto/`: Round-Trip Optimisation, C1 -> describe as Q2 -> C2 -> synthesise C3 when the code
  differs; fenced code-block extraction.
- `leap.py` -> `leap/`: extract few-shot examples from the query, generate intentional mistakes, derive
  principles from them, answer with the principles (separate temperature / token budget per phase).
- `plansearch.py` -> `plansearch/`: initial and derived observations -> natural-language solutions ->
  Python implementations.
- `pvg.py` -> `pvg/`: Prover-Verifier Game, helpful vs sneaky solutions, a verifier scores them, rounds
  refine the query.

They were ported rather than proxied so the techniques can run inside a Rust host with streaming
`optillm_core::ModelClient` calls, typed configs and unit tests on the extraction regexes.

Adaptation notes:

- The prompt texts and the step order are what matters: when upstream edits a template or adds/removes
  a phase, mirror it. Keep the Rust `*Config` structs (per-phase temperature and `max_tokens`) as the
  public surface; upstream's positional function args are not mirrored.
- Extraction uses the `regex` crate on the same tag / fence patterns upstream uses (`<output>`,
  triple-backtick code blocks, `Score:` lines). Tests in each `tests.rs` pin these patterns.
- RTO only synthesises C3 when `solutions_differ(C1, C2)` reports a difference, matching upstream's
  "if the two round-trip solutions agree, return C1" short-circuit.
- PlanSearch's final stage emits Python code strings exactly like upstream; there is no local execution.
- Token accounting is per call and summed into the returned `Solution` metadata.
