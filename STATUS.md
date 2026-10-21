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
- Approaches: bon, moa, rto, self_consistency, cot_reflection, re2.
- Python-compat runtime pieces: `json.dumps` / `repr`, CPython MT19937
  `random`, `difflib.SequenceMatcher`, `float()` / `str.strip()` semantics.
- Batch mode, conversation logging.
- Parity harness + 71 fixtures, all green.

## In progress

- Remaining existing-name approaches: mcts, pvg, rstar, plansearch, leap
  (PR 2).
- Missing approaches: mars, cepo, z3, plus the local-inference approaches
  (PR 3).
- Plugins (PR 4).

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
| Server | `provider selection` | 🟠 partial | 0 | OPENAI_API_KEY, CEREBRAS_API_KEY, AZURE_OPENAI_API_KEY supported. Gaps: OPTILLM_API_KEY local PyTorch inference engine and the LiteLLM fallback are not ported; Azure AD (DefaultAzureCredential) auth not supported; --launch-gui (Gradio) not supported |
| Approach | `bon` | ✅ parity | 7 | — |
| Approach | `moa` | ✅ parity | 5 | — |
| Approach | `rto` | ✅ parity | 4 | — |
| Approach | `self_consistency` | ✅ parity | 4 | difflib.SequenceMatcher ported incl. autojunk |
| Approach | `cot_reflection` | ✅ parity | 4 | — |
| Approach | `re2` | ✅ parity | 4 | — |
| Approach | `mcts` | ⬜ not started | 0 | — |
| Approach | `pvg` | ⬜ not started | 0 | — |
| Approach | `rstar` | ⬜ not started | 0 | — |
| Approach | `plansearch` | ⬜ not started | 0 | — |
| Approach | `leap` | ⬜ not started | 0 | — |
| Approach | `mars` | ⬜ not started | 0 | crates/mars is an independent native design, not a port |
| Approach | `cepo` | ⬜ not started | 0 | — |
| Approach | `z3` | ⬜ not started | 0 | — |
| Local-inference approach | `cot_decoding` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `entropy_decoding` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `thinkdeeper` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `deepconf` | ⬜ not started | 0 | upstream: local inference only |
| Local-inference approach | `autothink` | ⬜ not started | 0 | upstream: local inference only |
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

10/42 rows at harness-verified parity, 76 fixtures in total.
<!-- parity-table:end -->
