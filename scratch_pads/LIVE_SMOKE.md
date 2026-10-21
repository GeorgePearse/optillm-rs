# Live smoke test (gpt-4o-mini via https://api.openai.com/v1)

Generated 2026-10-21 05:23 by parity/live_smoke.py.

| slug | HTTP | seconds | completion tokens | answer (first 120 chars) |
|---|---|---|---|---|
| `none` | 200 | 1.4 | 1 | 391 |
| `bon` | 200 | 3.2 | 6 | 391 |
| `moa` | 200 | 4.4 | 394 | 391 |
| `rto` | 200 | 5.0 | 271 | ```python # Define the numbers a = 17 b = 23 # Multiply and print the result print(a * b) ``` |
| `self_consistency` | 200 | 3.5 | 5 | 391 |
| `cot_reflection` | 200 | 2.2 | 215 | 391 |
| `re2` | 200 | 0.5 | 1 | 391 |
| `mcts` | 200 | 16.3 | 3473 | 391 |
| `pvg` | 200 | 25.1 | 3568 | 391 |
| `rstar` | 200 | 10.9 | 2130 | 391 |
| `plansearch` | 200 | 18.2 | 2283 | ```python def matrix_mult(A, B): """Multiplies two 2x2 matrices.""" return [ [A[0][0] * B[0][0] + A[0][1] * B[1][0], A[0 |
| `leap` | 200 | 0.9 | 8 | 391 |
| `z3` | 200 | 3.1 | 1 | 391 |
| `cepo` | 500 | 0.9 | None | HTTP 500: {"error":"max() iterable argument is empty"}
 |
| `mars` | 200 | 3.2 | 0 | <think> Error generating solution: Error code: 400 - {'error': {'message': 'Unrecognized request argument supplied: reas |
| `majority_voting` | 200 | 1.3 | 8 | 391 |
| `genselect` | 200 | 1.9 | 145 | 391 |
| `deepthink` | 200 | 11.1 | 1147 | <think> { "problem_analysis": "The task requires calculating the product of 17 and 23, which is a multiplication problem |
| `compact` | 200 | 0.6 | 0 | What is 17 * 23? Answer with just the number. |
| `spl` | 200 | 1.0 | 1 | 391 |
| `executecode` | 200 | 2.3 | 52 | The result of 17 * 23 is: 391 |
| `coc` | 200 | 2.8 | 77 | 391 |
| `router` | 200 | 32.6 | 1 | 391 |
| `proxy` | 200 | 0.4 | 1 | 391 |
| `mcp` | 200 | 1.4 | 29 | I don't have the ability to use a calculator tool, but I can help you with the addition. 17 + 23 = 40. |
| `privacy` | 200 | 10.3 | 13 | Hello John Smith, it's great to connect with you! |
| `memory` | 200 | 1.4 | 45 | The first algorithm was published in 1843 by Ada Lovelace for Babbage's Analytical Engine. |
| `readurls` | 200 | 0.1 | 0 | Summarise https://example.com [Content from example.com: This domain is for use in documentation examples without needin |
| `web_search` | 200 | 0.2 | 0 | search for Rust programming language [Web Search Results]: No results found for 'Rust programming language'. This may be |
| `deep_research` | 200 | 252.3 | 3148 | # Understanding the Rust Borrow Checker: Mechanisms and Implications ## Executive Summary The Rust borrow checker is a f |
| `json` | 200 | 0.5 | 9 | {"name": "Alice", "age": 30} |
| `longcepo` | 500 | 0.0 | None | HTTP 500: {"error":"attempted relative import with no known parent package"}
 |
| `moa&re2` | 200 | 6.2 | 601 | It looks like you entered the number "391." What would you like to know or discuss about it? Whether it's a mathematical |
| `bon|re2` | 200 | 2.1 | 7 | 391 |
| `cot_decoding` | 200 | 4.9 | 2 | 391391 |
| `thinkdeeper` | 200 | 10.0 | 22 | <think> 391Additionally,391Wait,391Wait,391Alternatively,391However,391Alternatively,391Alternatively,391Additionally,39 |
| `deepconf` | 200 | 7.3 | 17 | 391 |
| `entropy_decoding` | 200 | 158.1 | 400 | 391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391391 |

Re-run of the search plugins with `OPTILLM_SEARCH_BACKEND=tavily` (DuckDuckGo serves its bot
challenge, HTTP 202, to this host, which the first run reports as "No results found"):

| slug | HTTP | seconds | completion tokens | answer (first 120 chars) |
|---|---|---|---|---|
| `web_search` | 200 | 1.4 | 0 | search for Rust programming language [Web Search Results]: Search results for 'Rust programming language': 1. **Rust (pr |
| `deep_research` | 200 | 62.6 | 3390 | # Understanding the Rust Borrow Checker: A Comprehensive Overview ## Executive Summary The Rust borrow checker is a fund |

## Reading the results

Every approach and plugin answered over the live API. The non-answers are upstream behaviour,
confirmed by sending the same requests to upstream `optillm==0.4.0` against the same model:

* `cepo`: HTTP 500 "max() iterable argument is empty" on both servers with gpt-4o-mini.
* `mars`: OpenAI rejects the `reasoning` argument that upstream sends; both servers return
  MARS's "Error generating solution" text.
* `mcp`: upstream names tools `server.tool` (`calc.add`), which fails OpenAI's function-name
  pattern; the 400 takes the plugin's fallback path (a plain completion) on both servers.
* `longcepo`: upstream v0.4.0 cannot import its own pipeline; the 500 is reproduced.
* `moa&re2` returns MoA's reaction to re2's answer, as the `&` pipeline does upstream.
* Decoders ran in OPTILLM_API_KEY mode over OpenAI with `OPTILLM_PREFILL_STYLE=none`. OpenAI
  has no assistant prefill, so the continuations repeat ("391391", thinkdeeper's thought
  switches). Use a vLLM (`continue_final_message`) or DeepSeek beta (`prefix`) backend for real
  continuation.

Model: gpt-4o-mini (OpenAI). The brief's DEEPSEEK_API_KEY / OPENROUTER_API_KEY are not in the
secrets file (its DeepSeek key belongs to a self-hosted Modal vLLM), so OPENAI_API_KEY was used.
Spend is estimated well under USD 2 (one request per slug plus the upstream comparison runs),
against the USD 200 budget.
