"""Generate parity/fixtures/**.json.

Fixtures are data; this script is the single place they are authored so
related cases share helpers. Re-run after editing:

    python parity/gen_fixtures.py

Each top-level `fx(...)` call writes parity/fixtures/<group>/<case>.json.
"""
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "fixtures"
FIXTURES = {}

SYS = "You are a helpful assistant."
Q = "What is 17 * 23? Answer with the number."


def chat(model="gpt-4o-mini", messages=None, **extra):
    body = {"model": model, "messages": messages or [{"role": "system", "content": SYS}, {"role": "user", "content": Q}]}
    body.update(extra)
    return {"method": "POST", "path": "/v1/chat/completions", "body": body}


def rule(response=None, responses=None, **match):
    r = {"match": match}
    if responses is not None:
        r["responses"] = responses
    else:
        r["response"] = response or {}
    return r


def fx(name, description, requests, rules=(), order="exact", server_args=(), env=None, seed=None, xfail=None, models=None,
       serialize_ms=None, pages=None):
    if isinstance(requests, dict):
        requests = [requests]
    data = {"description": description, "requests": list(requests), "mock": {"rules": list(rules)}, "order": order}
    if models is not None:
        data["mock"]["models"] = models
    if serialize_ms:
        data["mock"]["serialize_ms"] = serialize_ms
    if pages:
        data["mock"]["pages"] = pages
    if server_args:
        data["server_args"] = list(server_args)
    if env:
        data["env"] = env
    if seed is not None:
        data["seed"] = seed
    if xfail:
        data["xfail"] = xfail
    FIXTURES[name] = data


# --------------------------------------------------------------------------
# server surface
# --------------------------------------------------------------------------
fx("server/health", "GET /health", {"method": "GET", "path": "/health"})
fx("server/models", "GET /v1/models proxies the backend's model list through openai-python's Model schema",
   {"method": "GET", "path": "/v1/models"},
   models=[{"id": "m1", "object": "model", "created": 1, "owned_by": "me", "extra": True}, {"id": "m2"}])
fx("server/not_found", "unknown route", {"method": "GET", "path": "/nope"})
fx("server/auth_missing", "--optillm-api-key requires a bearer header", chat(), server_args=["--optillm-api-key", "secret"])
fx("server/auth_wrong", "wrong bearer key", dict(chat(), headers={"Authorization": "Bearer nope"}), server_args=["--optillm-api-key", "secret"])
fx("server/auth_ok", "right bearer key reaches the backend", dict(chat(), headers={"Authorization": "Bearer secret"}), server_args=["--optillm-api-key", "secret"])
fx("server/auth_health_exempt", "/health skips the key check", {"method": "GET", "path": "/health"}, server_args=["--optillm-api-key", "secret"])
fx("server/bad_content_type", "non-JSON content type", {"method": "POST", "path": "/v1/chat/completions", "raw_body": "hello", "headers": {"Content-Type": "text/plain"}})
fx("server/bearer_sk_passthrough", "a client bearer token starting with sk- replaces the server key",
   dict(chat(), headers={"Authorization": "Bearer sk-client-key"}))
fx("server/default_approach_flag", "--approach moa applies when the request names none", chat(), server_args=["--approach", "moa"])
fx("server/model_flag_default", "--model is used when the request has no model",
   {"method": "POST", "path": "/v1/chat/completions", "body": {"messages": [{"role": "user", "content": Q}]}},
   server_args=["--model", "my-default-model"])

# --------------------------------------------------------------------------
# none (passthrough)
# --------------------------------------------------------------------------
fx("none/basic", "plain passthrough returns the provider response verbatim (model_dump shape)", chat(),
   [rule({"content": "391"})])
fx("none/params", "sampling params pass straight through",
   chat(temperature=0.3, top_p=0.9, max_tokens=50, stop=["\n"], seed=7, presence_penalty=0.1, frequency_penalty=0.2, user="u1"),
   [rule({"content": "391"})])
fx("none/max_completion_tokens", "max_completion_tokens also sets max_tokens", chat(max_completion_tokens=77), [rule({"content": "391"})])
fx("none/n2", "n is passed through to the provider", chat(n=2), [rule({"contents": ["391", "three hundred ninety one"]})])
fx("none/stream", "stream=true re-chunks the full completion as SSE", chat(stream=True), [rule({"content": "391"})])
fx("none/stream_usage", "stream_options.include_usage adds a usage chunk",
   chat(stream=True, stream_options={"include_usage": True}), [rule({"content": "391"})])
fx("none/stream_tool_calls", "tool calls survive streaming",
   chat(stream=True, tools=[{"type": "function", "function": {"name": "mul", "parameters": {"type": "object"}}}]),
   [rule({"content": None, "finish_reason": "tool_calls",
          "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "mul", "arguments": "{\"a\": 17, \"b\": 23}"}}]})])
fx("none/tools_multiturn", "assistant tool_calls and tool results pass through intact",
   chat(messages=[
       {"role": "system", "content": SYS},
       {"role": "user", "content": Q},
       {"role": "assistant", "content": None, "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "mul", "arguments": "{}"}}]},
       {"role": "tool", "tool_call_id": "call_1", "content": "391"},
   ], tools=[{"type": "function", "function": {"name": "mul", "parameters": {"type": "object"}}}]),
   [rule({"content": "It is 391."})])
fx("none/list_content", "list-of-parts content is flattened to text",
   chat(messages=[{"role": "user", "content": [{"type": "text", "text": "What is"}, {"type": "text", "text": "17*23?"}]}]),
   [rule({"content": "391"})])
fx("none/reasoning_content", "extra message fields (reasoning_content) are kept",
   chat(stream=True), [rule({"content": "391", "reasoning_content": "17*23=391"})])
fx("none/unknown_kwarg", "an unknown body key is a TypeError in openai-python's create()",
   chat(mcts_simulations=3))
fx("none/optillm_approach_none", "optillm_approach=none in the body also reaches create() as a kwarg",
   chat(optillm_approach="none"))
fx("none/provider_400", "provider errors surface as 500 with openai-python's message",
   chat(), [rule({"status": 400, "body": {"error": {"message": "bad request", "type": "invalid_request_error", "code": None}}})])
fx("none/provider_500_retries", "5xx responses are retried twice like openai-python",
   chat(), [rule({"status": 503, "body": {"error": {"message": "overloaded"}}})])
fx("none/usage_details", "usage details objects survive normalisation",
   chat(), [rule({"content": "391", "usage": {"prompt_tokens": 5, "completion_tokens": 1, "total_tokens": 6,
                                                 "completion_tokens_details": {"reasoning_tokens": 0}, "prompt_tokens_details": {"cached_tokens": 2}}})])
fx("none/model_prefix", "an explicit none- prefix is stripped", chat(model="none-gpt-4o-mini"), [rule({"content": "391"})])
fx("none/unicode", "non-ASCII content is escaped exactly like Flask's jsonify",
   chat(messages=[{"role": "user", "content": "Grüße 👋 — 日本語"}]), [rule({"content": "Hallo 👋 — こんにちは"})])

# --------------------------------------------------------------------------
# approach selection (slug, extra_body, prompt tags, combinators)
# --------------------------------------------------------------------------
MOA_RULES = [
    rule({"contents": ["390", "391", "392"]}, n=3),
    rule({"content": "Candidate 2 is right."}, contains="Please provide your critique"),
    rule({"content": "391"}, contains="Please provide a final, optimized response"),
]
fx("slug/prefix_moa", "model-name slug prefix selects the approach", chat(model="moa-gpt-4o-mini"), MOA_RULES)
fx("slug/extra_body", "optillm_approach in the body selects the approach", chat(optillm_approach="moa"), MOA_RULES)
fx("slug/system_tag", "<optillm_approach> in the system prompt", chat(messages=[
    {"role": "system", "content": "<optillm_approach>moa</optillm_approach> " + SYS},
    {"role": "user", "content": Q}]), MOA_RULES)
fx("slug/user_tag", "<optillm_approach> in the user prompt", chat(messages=[
    {"role": "user", "content": Q + " <optillm_approach>re2</optillm_approach>"}]), [rule({"content": " 391 "})])
fx("slug/tag_overrides_body", "a prompt tag wins over optillm_approach in the body",
   chat(optillm_approach="moa", messages=[{"role": "user", "content": "<optillm_approach>re2</optillm_approach>" + Q}]),
   [rule({"content": "391"})])
fx("slug/multi_turn_query", "multi-turn conversations are flattened into User:/Assistant: lines",
   chat(model="re2-gpt-4o-mini", messages=[
       {"role": "system", "content": SYS},
       {"role": "user", "content": "Hi"},
       {"role": "assistant", "content": "Hello!"},
       {"role": "user", "content": Q}]), [rule({"content": "391"})])
fx("slug/untag_response", "approach output containing 'Assistant:' is reduced to the last turn",
   chat(model="re2-gpt-4o-mini"), [rule({"content": "User: what?\nAssistant: 391"})])
fx("slug/and_pipeline", "a&b runs approaches in sequence, feeding output forward",
   chat(model="re2&cot_reflection-gpt-4o-mini"),
   [rule({"content": "<thinking>17*23</thinking><output>391</output>"}, system_contains="Chain of Thought"),
    rule({"content": "I think 391"})])
fx("slug/or_parallel", "a|b runs approaches in parallel and returns one choice each",
   chat(model="re2|cot_reflection-gpt-4o-mini"),
   [rule({"content": "<output>391</output>"}, system_contains="Chain of Thought"), rule({"content": "391"})], order="any")
fx("slug/none_in_combo", "'none' cannot be combined", chat(model="none&re2-gpt-4o-mini"))
fx("slug/unknown_in_combo", "unknown names inside a combinator raise", chat(model="foo&re2-gpt-4o-mini"))
fx("slug/n2_approach", "n runs the approach n times", chat(model="re2-gpt-4o-mini", n=2), [rule({"content": "391"})])
fx("slug/stream_approach", "streaming an approach result", chat(model="re2-gpt-4o-mini", stream=True), [rule({"content": "391"})])
fx("slug/reasoning_tokens", "<think> blocks are counted into reasoning_tokens",
   chat(model="re2-gpt-4o-mini"), [rule({"content": "<think>seventeen times twenty three</think>391"})])
fx("slug/auto_model", "model 'auto' means no approach", chat(model="auto"), [rule({"content": "391"})])

# --------------------------------------------------------------------------
# bon
# --------------------------------------------------------------------------
BON_GEN = rule({"contents": ["390", "391", "392"]}, n=3, temperature=1)
fx("bon/basic", "n samples then one rating call per sample; highest rating wins", chat(model="bon-gpt-4o-mini"),
   [BON_GEN, rule(responses=[{"content": "4"}, {"content": "9"}, {"content": "7"}], contains="Rate the above response")])
fx("bon/tie_first_wins", "ties keep the first maximum", chat(model="bon-gpt-4o-mini"),
   [BON_GEN, rule(responses=[{"content": "8"}, {"content": " 8 "}, {"content": "2"}], contains="Rate the above response")])
fx("bon/unparseable_rating", "a non-numeric rating counts as 0", chat(model="bon-gpt-4o-mini"),
   [BON_GEN, rule(responses=[{"content": "Rating: 9"}, {"content": "5.5"}, {"content": "1_0"}], contains="Rate the above response")])
fx("bon/n_unsupported", "when n>1 fails the provider is called n times", chat(model="bon-gpt-4o-mini"),
   [rule({"status": 400, "body": {"error": {"message": "n is not supported"}}}, n=3),
    rule({"content": "6"}, contains="Rate the above response"),
    rule(responses=[{"content": "390"}, {"content": "391", "finish_reason": "length"}, {"content": "392"}], lacks=["n"])])
fx("bon/best_of_n_flag", "--best-of-n changes the sample count", chat(model="bon-gpt-4o-mini"),
   [rule({"contents": ["a", "b", "c", "d", "e"]}, n=5), rule({"content": "3"}, contains="Rate the above response")],
   server_args=["--best-of-n", "5"])
fx("bon/max_tokens", "request max_tokens flows into generation", chat(model="bon-gpt-4o-mini", max_tokens=123),
   [BON_GEN, rule({"content": "5"}, contains="Rate the above response")])
fx("bon/truncated_rating", "a truncated rating counts as 0", chat(model="bon-gpt-4o-mini"),
   [BON_GEN, rule(responses=[{"content": "9", "finish_reason": "length"}, {"content": "3"}, {"content": "2"}], contains="Rate the above response")])

# --------------------------------------------------------------------------
# moa
# --------------------------------------------------------------------------
fx("moa/basic", "3 candidates, critique, final synthesis", chat(model="moa-gpt-4o-mini"), MOA_RULES)
fx("moa/n_unsupported", "fallback generates candidates one by one", chat(model="moa-gpt-4o-mini"),
   [rule({"status": 400, "body": {"error": {"message": "n unsupported"}}}, n=3),
    rule({"content": "critique"}, contains="Please provide your critique"),
    rule({"content": "final"}, contains="Please provide a final"),
    rule(responses=[{"content": "c1"}, {"content": None}, {"content": "c3"}])])
fx("moa/padding", "fewer than 3 candidates are padded with the first", chat(model="moa-gpt-4o-mini"),
   [rule({"contents": ["only", None]}, n=3), rule({"content": "crit"}, contains="critique"), rule({"content": "fin"}, contains="final, optimized")])
fx("moa/critique_truncated", "truncated critique falls back to a generic critique", chat(model="moa-gpt-4o-mini"),
   [MOA_RULES[0], rule({"content": "cut", "finish_reason": "length"}, contains="Please provide your critique"), MOA_RULES[2]])
fx("moa/final_truncated", "truncated final answer falls back to candidate 1", chat(model="moa-gpt-4o-mini", max_tokens=64),
   [MOA_RULES[0], MOA_RULES[1], rule({"content": "partial", "finish_reason": "length"}, contains="Please provide a final")])

# --------------------------------------------------------------------------
# rto
# --------------------------------------------------------------------------
CODE1 = "```python\ndef f(x):\n    return x * 2\n```"
CODE2 = "```py\ndef f(x):\n    return 2 * x\n```"
fx("rto/converges", "identical round-trip code returns C1", chat(model="rto-gpt-4o-mini", messages=[{"role": "user", "content": "Write f doubling x"}]),
   [rule({"content": CODE1}, last_contains="Write f doubling x"), rule({"content": "Write a function f that doubles x."}, contains="Summarize or describe"),
    rule({"content": CODE1}, last_contains="Write a function f")])
fx("rto/diverges", "different round-trip code triggers a merge call", chat(model="rto-gpt-4o-mini", messages=[{"role": "user", "content": "Write f doubling x"}]),
   [rule({"content": CODE1}, last_contains="Write f doubling x"), rule({"content": "Write a function f that doubles x."}, contains="Summarize or describe"),
    rule({"content": CODE2}, last_contains="Write a function f"), rule({"content": "def f(x): return x + x"}, contains="Second generated code")])
fx("rto/no_fence", "unfenced code is used as-is", chat(model="rto-gpt-4o-mini"),
   [rule({"content": "plain code"}, last_contains=Q), rule({"content": "describe"}, contains="Summarize or describe"), rule({"content": "other code"}, last_contains="describe"),
    rule({"content": "merged"}, contains="Second generated code")])
fx("rto/truncated", "a truncated C1 is an error", chat(model="rto-gpt-4o-mini"), [rule({"content": "x", "finish_reason": "length"})])

# --------------------------------------------------------------------------
# self_consistency
# --------------------------------------------------------------------------
fx("self_consistency/majority", "5 samples clustered by difflib ratio; largest cluster wins", chat(model="self_consistency-gpt-4o-mini"),
   [rule(responses=[{"content": "The answer is 391."}, {"content": "I believe it's 400"}, {"content": "The answer is 391!"},
                    {"content": "The answer is 391"}, {"content": "I believe it's 400."}])])
fx("self_consistency/skips_truncated", "truncated / empty samples are skipped", chat(model="self_consistency-gpt-4o-mini"),
   [rule(responses=[{"content": "A", "finish_reason": "length"}, {"content": None}, {"content": "B"}, {"content": "C"}, {"content": "B"}])])
fx("self_consistency/long_autojunk", "long answers exercise difflib's autojunk heuristic", chat(model="self_consistency-gpt-4o-mini"),
   [rule(responses=[{"content": ("step " * 60) + "391"}, {"content": ("step " * 59) + "x 391"}, {"content": ("work " * 50) + "392"},
                    {"content": ("work " * 50) + "393"}, {"content": ("work " * 50) + "392"}])])
fx("self_consistency/all_bad", "no usable samples", chat(model="self_consistency-gpt-4o-mini"), [rule({"content": None})])

# --------------------------------------------------------------------------
# cot_reflection
# --------------------------------------------------------------------------
fx("cot_reflection/basic", "output tag is extracted", chat(model="cot_reflection-gpt-4o-mini"),
   [rule({"content": "<thinking>17*23\n<reflection>ok</reflection></thinking>\n<output>\n391\n</output>"})])
fx("cot_reflection/unclosed_output", "unclosed <output> runs to the end", chat(model="cot_reflection-gpt-4o-mini", temperature=0.2),
   [rule({"content": "<thinking>x</thinking><output>391\n"})])
fx("cot_reflection/no_tags", "no tags returns the whole response", chat(model="cot_reflection-gpt-4o-mini"), [rule({"content": "391"})])
fx("cot_reflection/full_response", "--return-full-response keeps the thinking", chat(model="cot_reflection-gpt-4o-mini"),
   [rule({"content": "<thinking>t</thinking><output>391</output>"})], server_args=["--return-full-response"])

# --------------------------------------------------------------------------
# re2
# --------------------------------------------------------------------------
fx("re2/basic", "re-reading prompt", chat(model="re2-gpt-4o-mini"), [rule({"content": "  391  "})])
fx("re2/max_tokens", "max_tokens only sent when provided", chat(model="re2-gpt-4o-mini", max_tokens=99), [rule({"content": "391"})])
fx("re2/server_n", "--n sets the provider n and returns a list", chat(model="re2-gpt-4o-mini"),
   [rule({"contents": ["391", "cut"], "finish_reason": ["stop", "length"]})], server_args=["--n", "2"])
fx("re2/truncated", "a truncated single answer is an error", chat(model="re2-gpt-4o-mini"), [rule({"content": "39", "finish_reason": "length"})])

# --------------------------------------------------------------------------
# batch mode
# --------------------------------------------------------------------------
BATCH = ["--batch-mode", "--batch-wait-ms", "10"]
fx("batching/none", "batched none: content is the whole provider completion dict", chat(), [rule({"content": "391"})], server_args=BATCH)
fx("batching/approach", "batched approach responses carry id/object/created/usage", chat(model="re2-gpt-4o-mini"),
   [rule({"content": "391"})], server_args=BATCH)
fx("batching/multi_turn", "batched none rebuilds messages from system prompt + flattened query",
   chat(messages=[{"role": "system", "content": SYS}, {"role": "user", "content": "Hi"}, {"role": "assistant", "content": "Hello"},
                  {"role": "user", "content": Q}]), [rule({"content": "391"})], server_args=BATCH)
fx("batching/stream_rejected", "streaming requests cannot be batched", chat(stream=True), server_args=BATCH)
fx("batching/error", "a failing request fails the batch with a wrapped message", chat(model="rto-gpt-4o-mini"),
   [rule({"content": "x", "finish_reason": "length"})], server_args=BATCH)

# --------------------------------------------------------------------------
# mcts (stochastic: seeded)
# --------------------------------------------------------------------------
MCTS_RULES = [
    rule({"contents": ["391", "It is 391.", "17*23 = 391"]}, n=3),
    rule(responses=[{"content": "Can you show the work?"}, {"content": "Why?"}, {"content": "Thanks!"},
                    {"content": "What about 17*24?"}, {"content": "Explain more"}, {"content": "OK"},
                    {"content": "Next?"}, {"content": "And 18*23?"}], contains="what might the user ask or say next"),
    rule(responses=[{"content": "0.8"}, {"content": "0.3"}, {"content": "0.9"}], contains="Evaluate the quality of this conversation"),
]
fx("mcts/basic", "2 simulations, depth 1 (server defaults)", chat(model="mcts-gpt-4o-mini"), MCTS_RULES, seed=0)
fx("mcts/seed_7", "same script, different seeded random path", chat(model="mcts-gpt-4o-mini"), MCTS_RULES, seed=7)
fx("mcts/request_params", "mcts_* request params override the server defaults", chat(model="mcts-gpt-4o-mini", mcts_simulations=3, mcts_depth=2, max_tokens=500),
   MCTS_RULES, seed=1)
fx("mcts/server_flags", "--mcts-* flags", chat(model="mcts-gpt-4o-mini"), MCTS_RULES, seed=3,
   server_args=["--mcts-simulations", "3", "--mcts-exploration", "1.0", "--mcts-depth", "2"])
fx("mcts/goodbye_terminal", "states whose next query says goodbye are terminal", chat(model="mcts-gpt-4o-mini"),
   [MCTS_RULES[0], rule({"content": "Goodbye!"}, contains="what might the user ask or say next"), MCTS_RULES[2]], seed=0)
fx("mcts/eval_parsing", "unparseable / out-of-range evaluations", chat(model="mcts-gpt-4o-mini"),
   [MCTS_RULES[0], MCTS_RULES[1], rule(responses=[{"content": "great"}, {"content": "1.7"}, {"content": "-2"}], contains="Evaluate the quality")], seed=2)

# --------------------------------------------------------------------------
# pvg
# --------------------------------------------------------------------------
PVG_RULES = [
    rule({"contents": ["Helpful: 17*23 = 391", "Helpful: it is 391", "Helpful: 391"]}, system_contains="", contains="You are in helpful mode."),
    rule({"contents": ["Sneaky: 17*23 = 401", "Sneaky: 381", "Sneaky: 391?"]}, contains="You are in sneaky mode."),
    rule(responses=[{"content": "Score: 9\nExplanation: correct"}, {"content": "Score: 7.5\nExplanation: ok"}, {"content": "Score: 6"},
                    {"content": "Score: 2\nExplanation: wrong"}, {"content": "no score here"}, {"content": "Score: 9.0\nExplanation: tie"},
                    {"content": "Score: 3"}, {"content": "Score: 10\nExplanation: best"}],
         system_contains="You are a verifier"),
    rule({"content": "What is 17 multiplied by 23, shown step by step?"}, contains="Refined query:"),
]
fx("pvg/basic", "2 rounds x (3 helpful + 3 sneaky) with verifier scores and query refinement", chat(model="pvg-gpt-4o-mini"), PVG_RULES)
fx("pvg/global_token_counter", "upstream's module-global token counter grows across requests (replicated)",
   [chat(model="pvg-gpt-4o-mini"), chat(model="pvg-gpt-4o-mini", max_tokens=300)], PVG_RULES)
fx("pvg/no_scores", "verifier replies without a Score line score 0", chat(model="pvg-gpt-4o-mini"),
   [PVG_RULES[0], PVG_RULES[1], rule({"content": "I cannot rate this."}, system_contains="You are a verifier"), PVG_RULES[3]])

# --------------------------------------------------------------------------
# rstar (stochastic + concurrent rollouts: seeded, mock releases requests in canonical order)
# --------------------------------------------------------------------------
RSTAR_RULES = [
    rule({"content": "Next step: multiply 17 by 23. The answer is 391"}, contains="Generate the next logical step"),
    rule({"content": "Continuing: 17*20 = 340 and 17*3 = 51, so 391."}, contains="Continue the reasoning process"),
    rule({"content": "Sub-question: what is 17*20? It is 340. Then add 51 to get 391."}, contains="Identify a key sub-question"),
    rule({"content": "Re-examining the step: 340 + 51 = 391. Thus, the answer is 391"}, contains="Re-examine the previous step"),
    rule({"content": "Rephrased: compute the product of 17 and 23."}, contains="Rephrase"),
    rule(responses=[{"content": "The final answer is 391"}, {"content": "multiply 17 by 23 the answer is 391"}, {"content": "no idea"}],
         contains="Complete the reasoning to solve the problem"),
]
fx("rstar/single_rollout", "one rollout (no concurrency)", chat(model="rstar-gpt-4o-mini"), RSTAR_RULES, seed=0,
   server_args=["--rstar-num-rollouts", "1", "--rstar-max-depth", "2"])
fx("rstar/default", "5 concurrent rollouts, depth 3", chat(model="rstar-gpt-4o-mini"), RSTAR_RULES, seed=0, order="any", serialize_ms=150)
fx("rstar/seed_11", "5 concurrent rollouts, another seed", chat(model="rstar-gpt-4o-mini"), RSTAR_RULES, seed=11, order="any", serialize_ms=150)
fx("rstar/no_numbers", "states without numbers give no valid answer", chat(model="rstar-gpt-4o-mini", messages=[{"role": "user", "content": "Name a colour"}]),
   [rule({"content": "Blue is a colour."})], seed=4, order="any", serialize_ms=150,
   server_args=["--rstar-num-rollouts", "2", "--rstar-max-depth", "1"])

# --------------------------------------------------------------------------
# plansearch
# --------------------------------------------------------------------------
PS_RULES = [
    rule({"content": "1. Use multiplication.\n\n2. 17*23 = 17*20 + 17*3\n 3. Check the result "}, contains="Please provide 3 observations."),
    rule({"content": "1. 17*20 = 340\n2. 17*3 = 51"}, contains="new observations derived"),
    rule({"content": "  Multiply and add: 340 + 51 = 391.  "}, contains="brainstorm a natural language solution"),
    rule({"content": "```python\nprint(17 * 23)\n```"}, contains="Please implement the solution in Python."),
]
fx("plansearch/basic", "observations -> derived observations -> solution -> code", chat(model="plansearch-gpt-4o-mini"), PS_RULES)
fx("plansearch/server_n", "--n 2 runs the plan search twice per pipeline run", chat(model="plansearch-gpt-4o-mini"), PS_RULES, server_args=["--n", "2"])
fx("plansearch/truncated_solution", "a truncated solution step is replaced by an error string", chat(model="plansearch-gpt-4o-mini", max_tokens=200),
   [PS_RULES[0], PS_RULES[1], rule({"content": "partial", "finish_reason": "length"}, contains="brainstorm a natural language solution"), PS_RULES[3]])
fx("plansearch/no_observations", "truncated observation calls yield empty lists", chat(model="plansearch-gpt-4o-mini"),
   [rule({"content": "cut", "finish_reason": "length"}, contains="observations"), PS_RULES[2], PS_RULES[3]])

# --------------------------------------------------------------------------
# leap
# --------------------------------------------------------------------------
FEWSHOT = "Q: 2+2? A: 4\nQ: 3+5? A: 8\nQ: 17*23? A:"
LEAP_EXAMPLES = rule({"content": '<output>[{"question": "2+2?", "answer": "4"}, {"question": "3+5?", "answer": "8"}]</output>'},
                     contains="determine if it contains few-shot examples")
LEAP_RULES = [
    LEAP_EXAMPLES,
    rule(responses=[{"content": "2+2 is 5. <output>5</output>"}, {"content": "3+5=8 <output>8</output>"}], contains="deliberately introduce an error"),
    rule({"content": "Reasoning: arithmetic slip. <output>Double-check additions.</output>"}, contains="Conduct a thorough analysis"),
    rule({"content": "<output>1. Double-check arithmetic.\n2. Verify each step.</output>"}, contains="Create a list of *unique*"),
    rule({"content": "391"}, contains="Keep in mind these principles"),
]
fx("leap/basic", "few-shot examples -> mistakes -> principles -> answer", chat(model="leap-gpt-4o-mini", messages=[{"role": "user", "content": FEWSHOT}]), LEAP_RULES)
fx("leap/no_examples", "no examples: answer directly with empty principles", chat(model="leap-gpt-4o-mini"),
   [rule({"content": "<output>[]</output>"}, contains="few-shot examples"), rule({"content": "391"}, contains="Keep in mind these principles")])
fx("leap/bad_json", "unparseable example JSON falls back to no examples", chat(model="leap-gpt-4o-mini"),
   [rule({"content": "<output>[{question: 1}]</output>"}, contains="few-shot examples"), rule({"content": "391"}, contains="Keep in mind")])
fx("leap/missing_keys", "examples without question/answer keys are dropped", chat(model="leap-gpt-4o-mini"),
   [rule({"content": '<output>[{"q": "2+2", "a": "4"}]</output>'}, contains="few-shot examples"), rule({"content": "391"}, contains="Keep in mind")])
fx("leap/non_string_answers", "numeric answers never equal the generated string answer", chat(model="leap-gpt-4o-mini", messages=[{"role": "user", "content": FEWSHOT}]),
   [rule({"content": '<output>[{"question": 2.5, "answer": 4}]</output>'}, contains="few-shot examples")] + LEAP_RULES[1:])
fx("leap/dict_json", "a JSON object instead of a list raises like Python's iteration", chat(model="leap-gpt-4o-mini"),
   [rule({"content": '<output>{"question": "x", "answer": "y"}</output>'}, contains="few-shot examples")])
fx("leap/truncated", "truncated responses raise", chat(model="leap-gpt-4o-mini"), [rule({"content": "x", "finish_reason": "length"})])

# --------------------------------------------------------------------------
# z3 (solver code is executed by CPython with z3 + sympy, like upstream)
# --------------------------------------------------------------------------
Z3_OK = "SOLVER_CAN_BE_APPLIED: True\n\nSOLVER_FORMULATION:\n```python\nx = Int('x')\ns = Solver()\ns.add(x == 17 * 23)\nprint(s.check())\nprint(s.model()[x])\n```\n\nAnalysis:\nmultiply"
Z3_BAD = "SOLVER_CAN_BE_APPLIED: True\n\nSOLVER_FORMULATION:\n```python\nprint(undefined_name)\n```\n"
Z3_SYNTAX = "SOLVER_CAN_BE_APPLIED: True\n\nSOLVER_FORMULATION:\n```python\ndef broken(:\n```\n"
fx("z3/not_applicable", "analysis says no solver: plain inference", chat(model="z3-gpt-4o-mini"),
   [rule({"content": "SOLVER_CAN_BE_APPLIED: False"}, contains="determine if it can be solved using Z3"), rule({"content": "391"})])
fx("z3/solves", "solver code runs and its stdout feeds the final answer", chat(model="z3-gpt-4o-mini"),
   [rule({"content": Z3_OK}, contains="determine if it can be solved using Z3"), rule({"content": "The answer is 391."}, contains="Solver Result:")])
fx("z3/fix_runtime_error", "a runtime error's traceback is sent back for a fix", chat(model="z3-gpt-4o-mini"),
   [rule({"content": Z3_BAD}, contains="determine if it can be solved using Z3"),
    rule({"content": "```python\nprint(17 * 23)\n```"}, contains="Fix the Z3 or SymPy code"),
    rule({"content": "391"}, contains="Solver Result:")])
fx("z3/syntax_errors_give_up", "three failed attempts fall back to plain inference", chat(model="z3-gpt-4o-mini"),
   [rule({"content": Z3_SYNTAX}, contains="determine if it can be solved using Z3"),
    rule({"content": "```python\nreturn (\n```"}, contains="Fix the Z3 or SymPy code"), rule({"content": "391"})])
fx("z3/no_code_block", "a missing code block falls back to plain inference", chat(model="z3-gpt-4o-mini"),
   [rule({"content": "SOLVER_CAN_BE_APPLIED: True but no code"}, contains="determine if it can be solved using Z3"), rule({"content": "391"})])

# --------------------------------------------------------------------------
# cepo (nested thread pools upstream: serialised mock, order-independent calls)
# --------------------------------------------------------------------------
CEPO_RULES = [
    rule({"content": "Plan: 1) multiply 17 by 20, 2) add 17*3. Confidence high."}, contains="come up with a concise plan"),
    rule({"content": "<think>working</think>  Executed: 340 + 51 = 391."}, last_contains="Can you execute the above plan"),
    rule({"content": "Final solution: 17*23 = 391."}, contains="identify any inconsistency between them"),
    rule({"content": "<think>check</think>The answer is 391."}, contains="Use your final solution from above"),
    rule({"content": "Explanation: correct\n\nRating: [[1]]"}, system_contains="impartial judge and evaluate"),
    rule({"content": "Response 1 is clearer. Better Response: [[1]]"}, system_contains="impartial judge and compare"),
]
SMALL = ["--cepo_bestofn_n", "2", "--cepo_planning_m", "3", "--cepo_planning_n", "2"]
fx("cepo/default_config", "default cepo_config.yaml: 3 completions x 6 plan attempts (first 3 kept), absolute rating",
   chat(model="cepo-gpt-4o-mini"), CEPO_RULES, order="any", serialize_ms=60)
fx("cepo/small", "--cepo_* overrides shrink the search", chat(model="cepo-gpt-4o-mini"), CEPO_RULES, order="any", serialize_ms=80,
   server_args=SMALL)
fx("cepo/plan_truncation", "length-truncated plan executions are dropped", chat(model="cepo-gpt-4o-mini"),
   [CEPO_RULES[0], rule(responses=[{"content": "cut", "finish_reason": "length"}, {"content": "Executed: 391."}], last_contains="Can you execute the above plan")]
   + CEPO_RULES[2:], order="any", serialize_ms=80, server_args=SMALL)
fx("cepo/no_reasoning", "--cepo_use_reasoning '' (argparse bool('') is False) drops reasoning_effort", chat(model="cepo-gpt-4o-mini"),
   CEPO_RULES, order="any", serialize_ms=80, server_args=SMALL + ["--cepo_use_reasoning", ""])
fx("cepo/think_value_error", "a 400 'think value' error disables reasoning for the rest of the process", chat(model="cepo-gpt-4o-mini"),
   [rule({"status": 400, "body": {"error": {"message": "think value high is not supported"}}}, has=["reasoning_effort"])] + CEPO_RULES,
   order="any", serialize_ms=80, server_args=["--cepo_bestofn_n", "1", "--cepo_planning_m", "1", "--cepo_planning_n", "1"])
fx("cepo/pairwise", "pairwise rating (config file)", chat(model="cepo-gpt-4o-mini"), CEPO_RULES, order="any", serialize_ms=80,
   server_args=["--cepo_config_file", "{PARITY_DIR}/fixture_data/cepo_pairwise.yaml"])
fx("cepo/majority", "majority vote over MCQ letters (config file)", chat(model="cepo-gpt-4o-mini"),
   [CEPO_RULES[0], CEPO_RULES[1], CEPO_RULES[2],
    # The parallel completions send identical requests, so their replies must be
    # identical too (which thread gets which reply is timing-dependent upstream).
    rule({"content": "Reasoning done.\n**Answer:** C"}, contains="Use your final solution from above")],
   order="any", serialize_ms=80, server_args=["--cepo_config_file", "{PARITY_DIR}/fixture_data/cepo_majority.yaml"])
fx("cepo/reasoning_fallback", "use_reasoning_fallback walks high -> medium on length", chat(model="cepo-gpt-4o-mini"),
   [rule({"content": "too long", "finish_reason": "length"}, contains="concise plan", has=["reasoning_effort"], **{}),
    rule(responses=[{"content": "cut", "finish_reason": "length"}, {"content": "Plan ok"}], contains="concise plan")] + CEPO_RULES[1:],
   order="any", serialize_ms=80, server_args=["--cepo_config_file", "{PARITY_DIR}/fixture_data/cepo_fallback.yaml"])

# --------------------------------------------------------------------------
# mars (asyncio + thread pool upstream: serialised mock, order-independent calls)
# --------------------------------------------------------------------------
MARS_EXPLORE = rule(responses=[
    {"content": "Let x = 17 * 23. Therefore x = 340 + 51 = 391.\n\n\\boxed{391}"},
    {"content": "We compute 17*23 = 391. Thus the product is \\boxed{391}."},
    {"content": "Possibly 17*23 is 381? I am not sure. The answer is 381."},
], contains="in a collaborative reasoning system")
MARS_COMMON = [
    MARS_EXPLORE,
    rule({"content": "Aggregated solution: 17 * 23 = 391, verified by 17*20 + 17*3. \\boxed{391}"}, system_contains="solution aggregation"),
    rule({"content": "PROBLEM_TYPE: algebra\nAPPROACH_TYPE: direct_computation\nKEY_INSIGHTS: distributivity, splitting 23 into 20+3\nMATHEMATICAL_TECHNIQUES: decomposition, addition\nSOLUTION_PATTERN: split a factor and add partial products\nSUCCESS_INDICATORS: simple arithmetic"},
         system_contains="strategy analysis expert"),
    rule({"content": " Algebra "}, system_contains="problem classifier"),
    rule({"content": "Enhanced: 17*20 = 340, 17*3 = 51, total 391. Therefore \\boxed{391}"}, system_contains="collaborative mathematical agent"),
    rule({"content": "Final synthesis: the product is \\boxed{391}."}, system_contains="mathematical synthesis expert"),
]
MARS_CORRECT = rule({"content": "Overall assessment: CORRECT. Every step checks out. Confidence: 9/10"}, contains="You are a verification expert")
MARS_INCORRECT = rule({"content": "Overall assessment: INCORRECT - there is an error in step 2. You should recompute. Confidence 4 out of 10"},
                      contains="You are a verification expert")
fx("mars/all_verified", "default config: exploration, 3 aggregation loops, strategy network, verification; a verified solution wins",
   chat(model="mars-gpt-4o-mini"), MARS_COMMON + [MARS_CORRECT], order="any", serialize_ms=60)
fx("mars/synthesis_vote", "nothing verifies: math_verify voting + synthesis prompt with extracted answers",
   chat(model="mars-gpt-4o-mini"), MARS_COMMON + [MARS_INCORRECT], order="any", serialize_ms=60)
fx("mars/lightweight", "max_tokens <= 4000 selects the lightweight config (2 agents, no aggregation / strategy network)",
   chat(model="mars-gpt-4o-mini", max_tokens=2000), MARS_COMMON + [MARS_INCORRECT], order="any", serialize_ms=60)
fx("mars/mars_config", "mars_config overrides in the request body", chat(model="mars-gpt-4o-mini", mars_config={"enable_aggregation": False, "num_agents": 2,
   "use_thinking_tags": False, "max_verification_attempts": 1}), MARS_COMMON + [MARS_INCORRECT], order="any", serialize_ms=60)
fx("mars/synthesis_formula", "string / set answers feed the EXTRACTED ANSWERS block of the synthesis prompt",
   chat(model="mars-gpt-4o-mini", mars_config={"enable_aggregation": False, "enable_strategy_network": False}),
   [rule(responses=[{"content": "Working it out, the answer is three hundred ninety-one."}, {"content": "We get \\boxed{\\{1, 2\\}} hmm"},
                    {"content": "x = 17 * 23 = 391\n"}], contains="in a collaborative reasoning system")] + MARS_COMMON[1:] + [MARS_INCORRECT],
   order="any", serialize_ms=60)
fx("mars/agent_errors", "provider errors during exploration become error solutions", chat(model="mars-gpt-4o-mini", mars_config={"enable_aggregation": False}),
   [rule({"status": 400, "body": {"error": {"message": "context too long"}}}, contains="in a collaborative reasoning system")] + MARS_COMMON[1:] + [MARS_CORRECT],
   order="any", serialize_ms=60)

# --------------------------------------------------------------------------
# local-inference decoders: with a remote provider upstream rejects them
# --------------------------------------------------------------------------
for name in ["cot_decoding", "entropy_decoding", "thinkdeeper", "deepconf", "autothink"]:
    fx(f"{name}/remote_provider_rejects", f"decoding={name} is a TypeError in openai-python's create() with a remote provider",
       chat(decoding=name))
    fx(f"{name}/not_a_slug", f"'{name}-' is not an approach slug: the whole string is the model name",
       chat(model=f"{name}-gpt-4o-mini"), [rule({"content": "391"})])

# --------------------------------------------------------------------------
# plugins
# --------------------------------------------------------------------------
fx("majority_voting/basic", "k=8 samples, normalised extracted answers, most common wins", chat(model="majority_voting-gpt-4o-mini"),
   [rule({"contents": ["The answer is 391.", "<think>hmm</think>Answer: 391", "Final answer: 390", "answer is 391!",
                       "B", "I think it is 391", "### 391", "Answer: 390"]}, n=8)])
fx("majority_voting/k_and_temperature", "k / temperature / max_tokens from the request", chat(model="majority_voting-gpt-4o-mini", k=3, temperature=0.2, max_tokens=99),
   [rule({"contents": ["A", "C", "C"]}, n=3)])
fx("majority_voting/sequential_fallback", "n unsupported -> k sequential calls", chat(model="majority_voting-gpt-4o-mini", k=3),
   [rule({"status": 400, "body": {"error": {"message": "n not supported"}}}, n=3), rule(responses=[{"content": "x"}, {"content": "y"}, {"content": "y"}])])
fx("majority_voting/nothing_extracted", "no extractable answers returns the first candidate", chat(model="majority_voting-gpt-4o-mini", k=2),
   [rule({"contents": ["...", "<think>only thinking</think>"]}, n=2)])

LONG_CONV = []
for i in range(6):
    LONG_CONV.append({"role": "user", "content": f"Turn {i}: please remember that file src/mod{i}.rs handles feature {i} and we decided to use option {i}."})
    LONG_CONV.append({"role": "assistant", "content": f"Noted: src/mod{i}.rs handles feature {i}; option {i} it is."})
LONG_CONV.append({"role": "user", "content": "Now summarise what we decided."})
fx("compact/passthrough", "short conversations pass through untouched", chat(model="compact-gpt-4o-mini"))
fx("compact/compress", "over budget: older turns summarised, 4 recent kept verbatim (context window from request config)",
   chat(model="compact-gpt-4o-mini", messages=[{"role": "system", "content": SYS}] + LONG_CONV, compact_context_window=100),
   [rule({"content": "<summary>\nConversation summary:\n- Scope: 9 earlier messages compacted.\n</summary>"}, system_contains="conversation summarizer")])
fx("compact/model_context_length", "context window read from models.retrieve()", chat(model="compact-gpt-4o-mini", messages=LONG_CONV),
   [rule({"content": "no tags here"}, system_contains="conversation summarizer")],
   models=[{"id": "gpt-4o-mini", "object": "model", "created": 1, "owned_by": "x", "context_length": 120}])
fx("compact/pipeline", "compact&re2: the compacted conversation feeds the next approach", chat(model="compact&re2-gpt-4o-mini",
   messages=LONG_CONV, compact_context_window=100, compact_keep_recent=2),
   [rule({"content": "<summary>short</summary>"}, system_contains="conversation summarizer"), rule({"content": "391"})])

fx("genselect/basic", "4 candidates, LLM comparison picks BEST CANDIDATE", chat(model="genselect-gpt-4o-mini"),
   [rule({"contents": ["390", "391 because 17*23", "392", None]}, n=4),
    rule({"content": "COMPARISON:\nok\n\nBEST CANDIDATE: 2\n\nREASONING:\nIt multiplies correctly."}, system_contains="expert evaluator")])
fx("genselect/include_reasoning", "include_reasoning appends the reasoning; fallback selection pattern", chat(model="genselect-gpt-4o-mini", num_candidates=2, include_reasoning=True),
   [rule({"contents": ["a", "b"]}, n=2), rule({"content": "I would select Candidate 2 here."}, system_contains="expert evaluator")])
fx("genselect/unparseable", "no selection found defaults to the first candidate", chat(model="genselect-gpt-4o-mini", num_candidates=1),
   [rule({"contents": ["first", "second"]}, n=2), rule({"content": "both fine"}, system_contains="expert evaluator")])
fx("genselect/sequential", "n unsupported -> sequential candidates", chat(model="genselect-gpt-4o-mini", num_candidates=2),
   [rule({"status": 400, "body": {"error": {"message": "no n"}}}, n=2), rule(responses=[{"content": "c1"}, {"content": "c2"}], lacks=["n"]),
    rule({"content": "BEST CANDIDATE: 9"}, system_contains="expert evaluator")])

DT_RULES = [
    rule({"content": "Selected: [9, 38, 39]"}, last_contains="Selected modules (JSON array only):"),
    rule({"content": "1. Break the multiplication into parts.\n2. Think step by step about partial products.\nnot numbered\n3. Plan and verify."}, last_contains="Provide the adapted modules as a numbered list:"),
    rule({"content": "Here you go:\n```json\n{\"decompose\": \"split 23 into 20 + 3\", \"multiply\": {\"a\": \"17*20\", \"b\": \"17*3\"}, \"answer\": \"sum\"}\n```"},
         last_contains="Valid JSON reasoning structure:"),
    rule({"content": "<think>17*20=340, 17*3=51</think>\nThe answer is 391"}, temperature=0.7),
    rule({"content": "<think>340+51</think>\nThe final answer is 391."}, temperature=0.0),
]
fx("deepthink/basic", "SELF-DISCOVER (select/adapt/implement) then uncertainty-routed CoT", chat(model="deepthink-gpt-4o-mini"), DT_RULES)
fx("deepthink/low_confidence", "inconsistent samples route to the greedy answer", chat(model="deepthink-gpt-4o-mini", deepthink_samples=3),
   DT_RULES[:3] + [rule(responses=[{"content": "<think>a</think> answer is 1"}, {"content": "<think>zzzz</think> answer is 99999"}, {"content": "nothing"}], temperature=0.7), DT_RULES[4]])
fx("deepthink/no_self_discover", "enable_self_discover=false skips structure discovery", chat(model="deepthink-gpt-4o-mini", enable_self_discover=False, deepthink_samples=2, temperature=1.5, top_p=0.5, max_tokens=300),
   [rule({"content": "<think>x</think>Thus 391"}, temperature=1.5), DT_RULES[4]])
fx("deepthink/bad_json_structure", "unparseable structures fall back to the default structure", chat(model="deepthink-gpt-4o-mini"),
   DT_RULES[:2] + [rule({"content": "{'step': 'one', bad: two,}"}, last_contains="Valid JSON reasoning structure:")] + DT_RULES[3:])

MEM_DOC = ("Ada Lovelace wrote the first published algorithm for Babbage's Analytical Engine. "
           "She was born in 1815 in London. Charles Babbage designed the engine. Query: When was Ada Lovelace born?")
fx("memory/basic", "context is mined for margin notes, then the query is answered from them", chat(model="memory-gpt-4o-mini",
   messages=[{"role": "user", "content": MEM_DOC}]),
   [rule({"content": "YES#Ada Lovelace was born in 1815 in London."}, last_contains="Copy over all context relevant"), rule({"content": "1815"})])
fx("memory/no_relevant", "NO# margins leave an empty note list", chat(model="memory-gpt-4o-mini",
   messages=[{"role": "user", "content": MEM_DOC}]), [rule({"content": "NO#No relevant context."}, last_contains="Copy over all context"), rule({"content": "?"})])
fx("memory/sentence_split", "without 'Query:' the last sentence becomes the query", chat(model="memory-gpt-4o-mini",
   messages=[{"role": "user", "content": "Paris is in France. Berlin is in Germany! Which city is in France?"}]),
   [rule({"content": "YES#Paris is in France."}, last_contains="Copy over all context"), rule({"content": "Paris"})])
fx("memory/persisted_ranking", "OPTILLM_MEMORY_FILE persists notes; TF-IDF ranks them for later queries",
   [chat(model="memory-gpt-4o-mini", messages=[{"role": "user", "content": "Cats purr. Query: what do cats do?"}]),
    chat(model="memory-gpt-4o-mini", messages=[{"role": "user", "content": "Dogs bark loudly at night. Query: what do dogs do at night?"}]),
    chat(model="memory-gpt-4o-mini", messages=[{"role": "user", "content": "Birds sing. Query: what do dogs and birds do?"}])],
   [rule({"content": "YES#Cats purr when happy."}, contains="Cats purr. "), rule({"content": "YES#Dogs bark loudly at night."}, contains="Dogs bark"),
    rule({"content": "YES#Birds sing in the morning."}, contains="Birds sing. "), rule({"content": "ok"})],
   env={"OPTILLM_MEMORY_FILE": "{SERVER_TMP}/memory.json"})

SPL_CLASSIFY = rule({"content": "<think>numbers</think>arithmetic_calculation"}, system_contains="You are a problem classifier")
fx("spl/inference_empty_db", "inference mode with no strategies: classify, then the base prompt", chat(model="spl-gpt-4o-mini"),
   [SPL_CLASSIFY, rule({"content": "391"})])
fx("spl/learning", "learning mode creates, evaluates and reuses strategies across requests",
   [chat(model="spl-gpt-4o-mini", spl_learning=True),
    chat(model="spl-gpt-4o-mini", spl_learning=True, messages=[{"role": "user", "content": "What is 12 * 13? Answer with the number."}]),
    chat(model="spl-gpt-4o-mini", max_tokens=100)],
   [SPL_CLASSIFY,
    rule({"content": "<think>plan</think>1. Split numbers into tens and units.\n2. Multiply the parts.\n3. Add the partial products."}, system_contains="expert problem-solving strategist"),
    rule({"content": "<think>no</think>YES"}, system_contains="evaluating the effectiveness"),
    rule({"content": "<think>17*20+17*3</think>391"})])
fx("spl/kwarg_fallback", "an unknown body key breaks the main call; the plain fallback answers", chat(model="spl-gpt-4o-mini", foo="bar"),
   [SPL_CLASSIFY, rule({"content": "391"})])

CODE_OK = "Here:\n```python\nanswer = 17 * 23\nprint(answer)\n```"
CODE_BAD = "```python\nanswer = 17 * undefined_value\n```"
fx("executecode/generated_code", "the model's code block runs in a Jupyter kernel and the output is fed back", chat(model="executecode-gpt-4o-mini"),
   [rule({"content": "```python\nx = 17 * 23\nprint(x)\nx + 1\n```"}, system_contains="Put the code in a ```python block"),
    rule({"content": "The answer is 391."}, contains="Based on the code execution output")])
fx("executecode/no_code", "no code block: the first answer is returned", chat(model="executecode-gpt-4o-mini"), [rule({"content": "391"})])
fx("executecode/request_code_unpacks", "a python block in the request hits upstream's tuple-unpacking bug",
   chat(model="executecode-gpt-4o-mini", messages=[{"role": "user", "content": "Run this:\n```python\nprint(1)\n```"}]))
fx("executecode/runtime_error", "a failing cell raises (CellExecutionError) like upstream", chat(model="executecode-gpt-4o-mini"),
   [rule({"content": "```python\nraise ValueError('boom')\n```"})])
fx("coc/executes", "chain of code: generated code runs and `answer` is returned", chat(model="coc-gpt-4o-mini"),
   [rule({"content": CODE_OK})])
fx("coc/fix_then_run", "an execution error is sent back for a fix", chat(model="coc-gpt-4o-mini"),
   [rule({"content": CODE_BAD}, system_contains="Write Python code to solve this problem"),
    rule({"content": "```python\nanswer = [17 * 23, 'ok']\n```"}, system_contains="failed to execute")])
fx("coc/simulate", "three failed executions fall back to LLM simulation (literal_eval'd)", chat(model="coc-gpt-4o-mini"),
   [rule({"content": CODE_BAD}, system_contains="Write Python code"), rule({"content": CODE_BAD}, system_contains="failed to execute"),
    rule({"content": " {'a': 3.0, 'b': (1, 2)} "}, system_contains="could not be executed directly")])
fx("coc/no_code", "no code block returns the model text", chat(model="coc-gpt-4o-mini"), [rule({"content": "just 391"})])

PAGES = {
    "article": {"html": "<html><head><style>p{}</style></head><body><nav><p>menu</p></nav><article><h2>Ada</h2>"
                        "<p>Ada Lovelace wrote the <em>first</em> algorithm.[3]</p><script>track()</script>"
                        "<table><tr><th>Year</th><th>Event</th></tr><tr><td>1843</td><td>Notes</td></tr></table></article></body></html>"},
    "plain": {"html": "<html><body><h1>Plain page</h1><p>Line one.</p>\n<p>Line   two.</p></body></html>"},
    "missing": {"status": 404, "html": "nope"},
}
fx("readurls/fetch", "URLs in the query are fetched and their text spliced in (no LLM call)",
   chat(model="readurls-gpt-4o-mini", messages=[{"role": "user", "content": "Summarise {MOCK}/__page/article and {MOCK}/__page/plain."}]), pages=PAGES)
fx("readurls/http_error", "HTTP errors are reported inline like requests.raise_for_status()",
   chat(model="readurls-gpt-4o-mini", messages=[{"role": "user", "content": "Read '{MOCK}/__page/missing'"}]), pages=PAGES)
fx("readurls/pipeline", "readurls&re2: fetched content feeds the next approach",
   chat(model="readurls&re2-gpt-4o-mini", messages=[{"role": "user", "content": "Who is in {MOCK}/__page/article?"}]), [rule({"content": "Ada"})], pages=PAGES)

fx("web_search/no_query", "a bare search command has nothing to search: the query passes through",
   chat(model="web_search-gpt-4o-mini", messages=[{"role": "user", "content": "search for"}]))
fx("web_search/too_short", "queries shorter than 3 characters are not searched",
   chat(model="web_search-gpt-4o-mini", messages=[{"role": "user", "content": "hi"}]))

fx("json/no_schema", "no response_format schema: plain completion", chat(model="json-gpt-4o-mini"), [rule({"content": "391"})])
fx("json/non_schema_format", "response_format json_object is not a schema: plain completion", chat(model="json-gpt-4o-mini", response_format={"type": "json_object"}),
   [rule({"content": "{\"a\": 1}"})])
fx("json/schema", "with a schema upstream generates locally (Qwen2.5-Coder-0.5B via outlines; offline here, so it falls back)",
   chat(model="json-gpt-4o-mini", response_format={"type": "json_schema", "json_schema": {"name": "x", "schema": {"type": "object", "properties": {"answer": {"type": "integer"}}, "required": ["answer"]}}}),
   [rule({"content": "{\"answer\": 391}"})],
   xfail="upstream uses a local outlines model for schema-constrained output; optillm-rs asks the provider for structured output")

fx("privacy/anonymize_roundtrip", "PII is replaced with <TYPE_n> placeholders before the LLM call and restored after",
   chat(model="privacy-gpt-4o-mini", messages=[{"role": "user", "content": "Write a short note to John Smith at john.smith@example.com about the meeting in Paris."}]),
   [rule({"content": "Dear <PERSON_0>, see you in <LOCATION_0>. Reply to <EMAIL_ADDRESS_0>. Unknown <PHONE_NUMBER_7>."})])

fx("router/model_unavailable", "with the router model unavailable (offline hub) both fall back to one direct completion",
   chat(model="router-gpt-4o-mini", max_tokens=50), [rule({"content": "391"})])


def main():
    if ROOT.exists():
        shutil.rmtree(ROOT)
    for name, data in FIXTURES.items():
        path = ROOT / f"{name}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {len(FIXTURES)} fixtures to {ROOT}")


if __name__ == "__main__":
    main()
