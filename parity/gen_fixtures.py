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


def fx(name, description, requests, rules=(), order="exact", server_args=(), env=None, seed=None, xfail=None, models=None):
    if isinstance(requests, dict):
        requests = [requests]
    data = {"description": description, "requests": list(requests), "mock": {"rules": list(rules)}, "order": order}
    if models is not None:
        data["mock"]["models"] = models
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
