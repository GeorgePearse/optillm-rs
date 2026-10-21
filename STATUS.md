# optillm parity status

Goal (George, 2026-10-21): make optillm-rs a fully functional, call-for-call
match of upstream [codelion/optillm](https://github.com/codelion/optillm).

- **Parity target:** optillm v0.4.0, commit `e4ad19906578c9c472a3f8fd04d218db89a31c8c`
  (PyPI `optillm==0.4.0`; the installed package was diffed against the tag and
  matches apart from READMEs and sample data).
- **Implementation:** `crates/proxy` (`optillm-proxy` binary, `optillm serve`).
- **Verification:** `parity/run_parity.py` runs upstream and Rust against the
  same recording mock backend; CI workflow `.github/workflows/parity.yml`.

## Done

- OpenAI-compatible server: `/v1/chat/completions` (JSON + SSE), `/v1/models`,
  `/health`, bearer-key auth, Flask/werkzeug-identical bodies.
- All upstream CLI flags and env vars (`--approach`, `--n`, `--base-url`,
  `--best-of-n`, `--mcts-*`, `--rstar-*`, `--return-full-response`, `--port`,
  `--host`, `--log`, `--ssl-*`, `--batch-*`, `--log-conversations`,
  `--cepo_*`), with argparse's env-default semantics.
- Approach selection: slug prefix, `optillm_approach`, `<optillm_approach>`
  tags, `&` / `|` combinators, `n` repeats, response untagging,
  reasoning-token counting.
- `none` passthrough with openai-python `model_dump` normalisation, kwarg
  validation and retry policy.
- Approaches: bon, moa, rto, self_consistency, cot_reflection, re2 (PR #2);
  mcts, pvg, rstar, plansearch, leap (PR 2 of the stack).
- z3, cepo, mars (upstream's optillm/mars, not crates/mars) at harness parity (PR 3).
- Local-inference decoders: remote-provider behaviour at parity; `OPTILLM_API_KEY`
  mode emulates cot_decoding / thinkdeeper / deepconf / entropy_decoding over API
  logprobs + assistant prefill (`OPTILLM_PREFILL_STYLE=vllm|deepseek|none`);
  autothink is a documented gap.
- Stochastic approaches are compared under a fixed seed (CPython MT19937 port
  plus per-request reseeding of upstream). Concurrent ones (rstar) use the mock's
  serialised mode so both servers resume rollouts in the same order.
- Python-compat runtime pieces: `json.dumps` / `repr`, CPython MT19937
  `random`, `difflib.SequenceMatcher`, `float()` / `str.strip()` semantics.
- Batch mode, conversation logging.
- Parity harness, all fixtures green (see the table for counts).

## In progress

- Plugins (PR 4).
- One live end-to-end smoke test per approach against a cheap model.

## Housekeeping notes

- `crates/mars` (the original native-API strategies) failed
  `clippy -D warnings` and one example no longer compiled. Both are fixed so the
  workspace-wide prek clippy hook passes. Three `crates/mars` unit tests already
  failed on `main` before this work. They are marked `#[ignore]` with that
  reason rather than changed.

## Parity table

<!-- parity-table:start -->
| Kind | Name | Status | Fixtures | Known deviations / notes |
|---|---|---|---|---|
| Server | `proxy surface` | ✅ parity | 11 | /v1/chat/completions, /v1/models, /health, bearer-key check, Flask jsonify byte format, werkzeug 404/415 pages |
| Server | `approach selection` | ✅ parity | 15 | slug prefix, optillm_approach in body, <optillm_approach> tags, & pipeline, \| parallel, n repeats, untagging of User:/Assistant: output |
| Server | `none (passthrough)` | ✅ parity | 17 | openai-python model_dump shape, create() kwarg TypeError, 2 retries on 5xx/429, SSE re-chunking incl. tool calls and usage |
| Server | `batching` | ✅ parity | 5 | --batch-mode/--batch-size/--batch-wait-ms; per model:approach queues, sequential processing, wrapped error messages |
| Server | `conversation logging` | 🟡 implemented, not yet harness-verified | 0 | --log-conversations JSONL ported; not compared by the harness (timestamps, uuids) |
| Server | `provider selection` | 🟠 partial | 0 | OPENAI_API_KEY, CEREBRAS_API_KEY, AZURE_OPENAI_API_KEY supported. OPTILLM_API_KEY (upstream: in-process PyTorch engine) is emulated on the configured provider (see decoder rows). Gaps: LiteLLM fallback not ported; Azure AD (DefaultAzureCredential) auth not supported; --launch-gui (Gradio) not supported |
| Approach | `bon` | ✅ parity | 7 | — |
| Approach | `moa` | ✅ parity | 5 | — |
| Approach | `rto` | ✅ parity | 4 | — |
| Approach | `self_consistency` | ✅ parity | 4 | difflib.SequenceMatcher ported incl. autojunk |
| Approach | `cot_reflection` | ✅ parity | 4 | — |
| Approach | `re2` | ✅ parity | 4 | — |
| Approach | `mcts` | ✅ parity | 6 | seeded: CPython MT19937 random.choice; numpy log/sqrt replaced by f64 (only affects exact UCB ties) |
| Approach | `pvg` | ✅ parity | 3 | upstream module-global token counter (never reset, grows across requests) replicated |
| Approach | `rstar` | ✅ parity | 4 | concurrent asyncio rollouts reproduced; harness serialises mock replies so seeded draws line up |
| Approach | `plansearch` | ✅ parity | 4 | — |
| Approach | `leap` | ✅ parity | 7 | Python json/iteration error semantics for malformed example JSON |
| Approach | `mars` | ✅ parity | 6 | port of optillm/mars (not crates/mars). Shared executor = FIFO semaphore; str-vs-int agent-id quirk kept (agent 0 verifies all, improvement never runs); math_verify fails in upstream's server threads (signal-based timeout) so voting uses the regex fallback natively; agg_HHMMSS ids normalised by the harness; CPython 3.12 Neumaier sum() ported |
| Approach | `cepo` | ✅ parity | 8 | nested thread pools + as_completed reproduced; process-global mutable config; --cepo_bestofn_rating_type / --cepo_rating_model accepted as str (upstream argparse rejects them, use --cepo_config_file); majority vote uses the MCQ path only (math_verify is unreachable with non-empty completions) |
| Approach | `z3` | ✅ parity | 5 | solver code runs in CPython+z3+sympy subprocess ($OPTILLM_Z3_PYTHON) with upstream exec globals; harness normalises the exec frame path in tracebacks |
| Local-inference approach | `cot_decoding` | 🟣 remote mode at parity; local mode emulated over API logprobs | 2 | Remote provider: identical TypeError for decoding=, slug not an approach. OPTILLM_API_KEY mode: same algorithm over logprobs + assistant prefill (k capped at 20 top logprobs; num_beams/length_penalty/no_repeat_ngram_size unavailable) |
| Local-inference approach | `entropy_decoding` | 🟣 remote mode at parity; local mode emulated over API logprobs | 2 | Remote: parity. Local mode: logits-only approximation (1 call/token, top-20 logprobs, attention metrics = 0, no clarification-token insertion) |
| Local-inference approach | `thinkdeeper` | 🟣 remote mode at parity; local mode emulated over API logprobs | 2 | Remote: parity. Local mode: same control loop, chunked with stop=[end_think_token]; max_thoughts checked between chunks |
| Local-inference approach | `deepconf` | 🟣 remote mode at parity; local mode emulated over API logprobs | 2 | Remote: parity. Local mode: warmup/threshold/consensus/weighted vote over top-k logprobs; early termination applied post hoc |
| Local-inference approach | `autothink` | 🟠 partial | 2 | Remote: parity. Local mode: NOT available (needs in-process complexity classifier + steering vectors on hidden states) |
| Plugin | `coc` | ⬜ not started | 0 | — |
| Plugin | `compact` | ⬜ not started | 0 | — |
| Plugin | `deep_research` | ⬜ not started | 0 | — |
| Plugin | `deepthink` | ⬜ not started | 0 | — |
| Plugin | `executecode` | ⬜ not started | 0 | — |
| Plugin | `genselect` | ⬜ not started | 0 | — |
| Plugin | `json` | ⬜ not started | 0 | — |
| Plugin | `longcepo` | ⬜ not started | 0 | — |
| Plugin | `majority_voting` | ⬜ not started | 0 | — |
| Plugin | `mcp` | ⬜ not started | 0 | — |
| Plugin | `memory` | ⬜ not started | 0 | — |
| Plugin | `privacy` | ⬜ not started | 0 | — |
| Plugin | `proxy` | ⬜ not started | 0 | — |
| Plugin | `readurls` | ⬜ not started | 0 | — |
| Plugin | `router` | ⬜ not started | 0 | — |
| Plugin | `spl` | ⬜ not started | 0 | — |
| Plugin | `web_search` | ⬜ not started | 0 | — |

18/42 rows at harness-verified parity, 129 fixtures in total.
<!-- parity-table:end -->
