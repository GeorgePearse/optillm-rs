"""Live end-to-end smoke test: one request per approach / plugin against a real provider.

Starts optillm-proxy on a free port with a throwaway HOME, sends one request per
slug and writes a Markdown table. Provider keys are read from the environment
only (never written anywhere).

    OPENAI_API_KEY=... python parity/live_smoke.py --rust-bin target/release/optillm-proxy \
        --base-url https://api.deepseek.com/v1 --model deepseek-chat \
        --python ~/optillm-upstream/venv/bin/python --out scratch_pads/LIVE_SMOKE.md

Decoding approaches (cot_decoding, thinkdeeper, deepconf, entropy_decoding) run on a
second server in OPTILLM_API_KEY mode when --local-base-url is given (a backend with
logprobs and assistant prefill, e.g. https://api.deepseek.com/beta with
--prefill-style deepseek).
"""
import argparse
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
RESERVED = {8090, 8091, 8094}
Q = "What is 17 * 23? Answer with just the number."
CONTEXT = ("Ada Lovelace wrote the first published algorithm for Babbage's Analytical Engine in 1843. " * 40)

MCP_SERVER = str(HERE / "fixture_data" / "mcp_server.py")


def free_port():
    while True:
        with socket.socket() as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        if port not in RESERVED:
            return port


def post(url, body, key, timeout=900):
    req = urllib.request.Request(url, data=json.dumps(body).encode(), method="POST",
                                 headers={"Content-Type": "application/json", "Authorization": f"Bearer {key}"})
    t0 = time.time()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, json.loads(r.read()), time.time() - t0
    except urllib.error.HTTPError as e:
        return e.code, {"error": e.read().decode()[:300]}, time.time() - t0
    except Exception as e:  # timeouts, resets
        return 0, {"error": repr(e)[:300]}, time.time() - t0


def cases(model):
    user = [{"role": "user", "content": Q}]
    c = {}
    for slug in ["none", "bon", "moa", "rto", "self_consistency", "cot_reflection", "re2", "mcts", "pvg", "rstar",
                 "plansearch", "leap", "z3", "cepo", "mars", "majority_voting", "genselect", "deepthink", "compact",
                 "spl", "executecode", "coc", "router", "proxy", "mcp", "privacy"]:
        c[slug] = {"model": f"{slug}-{model}", "messages": user}
    c["plansearch"]["messages"] = [{"role": "user", "content": "Write a Python function that returns the n-th Fibonacci number."}]
    c["executecode"]["messages"] = [{"role": "user", "content": "Compute 17 * 23 with Python code and give the result."}]
    c["mcp"]["messages"] = [{"role": "user", "content": "Use the calculator tool to add 17 and 23."}]
    c["privacy"]["messages"] = [{"role": "user", "content": "My name is John Smith and my email is john@example.com. Write me a one-line greeting."}]
    c["memory"] = {"model": f"memory-{model}", "messages": [{"role": "user", "content": CONTEXT + "\nQuery: When was the first algorithm published?"}]}
    c["readurls"] = {"model": f"readurls-{model}", "messages": [{"role": "user", "content": "Summarise https://example.com in one sentence."}]}
    c["web_search"] = {"model": f"web_search-{model}", "messages": [{"role": "user", "content": "search for Rust programming language"}],
                       "num_results": 3, "delay_seconds": 0}
    c["deep_research"] = {"model": f"deep_research-{model}", "messages": [{"role": "user", "content": "What is the Rust borrow checker?"}],
                          "max_iterations": 1, "max_sources": 4}
    c["json"] = {"model": f"json-{model}", "messages": [{"role": "user", "content": "Give the name and age of a fictional person."}],
                 "response_format": {"type": "json_schema", "json_schema": {"name": "p", "schema": {
                     "type": "object", "properties": {"name": {"type": "string"}, "age": {"type": "integer"}},
                     "required": ["name", "age"], "additionalProperties": False}}}}
    c["longcepo"] = {"model": f"longcepo-{model}", "messages": [{"role": "user", "content": "Ctx.<CONTEXT_END>Q?"}]}
    c["moa&re2"] = {"model": f"re2&moa-{model}", "messages": user}
    c["bon|re2"] = {"model": f"bon|re2-{model}", "messages": user}
    return c


def start(rust_bin, env, args, log):
    port = free_port()
    proc = subprocess.Popen([rust_bin, "--port", str(port)] + args, env=env, stdout=log, stderr=subprocess.STDOUT)
    for _ in range(300):
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=1)
            return proc, port
        except Exception:
            if proc.poll() is not None:
                raise RuntimeError("server exited")
            time.sleep(0.1)
    raise RuntimeError("server did not start")


def summarize(status, body):
    if status != 200:
        return f"HTTP {status}: {str(body.get('error'))[:120]}"
    content = (body.get("choices") or [{}])[0].get("message", {}).get("content") or ""
    return " ".join(str(content).split())[:120]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rust-bin", required=True)
    ap.add_argument("--base-url", required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--python", default=sys.executable, help="python with upstream's deps (z3, nbconvert, presidio, mcp)")
    ap.add_argument("--local-base-url")
    ap.add_argument("--prefill-style", default="deepseek")
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    key = os.environ["OPENAI_API_KEY"]

    home = tempfile.mkdtemp(prefix="optillm-smoke-")
    os.makedirs(f"{home}/.optillm", exist_ok=True)
    with open(f"{home}/.optillm/mcp_config.json", "w") as fh:
        json.dump({"mcpServers": {"calc": {"command": args.python, "args": [MCP_SERVER], "description": "Calculator"}}}, fh)
    with open(f"{home}/.optillm/proxy_config.yaml", "w") as fh:
        fh.write(f"providers:\n  - name: main\n    base_url: {args.base_url}\n    api_key: ${{OPENAI_API_KEY}}\n"
                 "routing:\n  strategy: round_robin\n  health_check:\n    enabled: false\n")
    env = {**os.environ, "HOME": home, "OPTILLM_PYTHON": args.python, "OPTILLM_SPL_DATA_DIR": f"{home}/spl"}
    env.pop("OPTILLM_API_KEY", None)
    log = open(f"{home}/server.log", "w")
    rows = []
    proc, port = start(args.rust_bin, env, ["--base-url", args.base_url], log)
    try:
        for name, body in cases(args.model).items():
            if args.only and name not in args.only:
                continue
            status, resp, secs = post(f"http://127.0.0.1:{port}/v1/chat/completions", body, key)
            tokens = (resp.get("usage") or {}).get("completion_tokens")
            rows.append((name, status, secs, tokens, summarize(status, resp)))
            print(f"{name:18} {status} {secs:6.1f}s  {rows[-1][4]}", flush=True)
    finally:
        proc.terminate()
    if args.local_base_url:
        lenv = {**env, "OPTILLM_API_KEY": "smoke-local", "OPTILLM_PREFILL_STYLE": args.prefill_style}
        proc, port = start(args.rust_bin, lenv, ["--base-url", args.local_base_url], log)
        try:
            for dec in ["cot_decoding", "thinkdeeper", "deepconf", "entropy_decoding"]:
                if args.only and dec not in args.only:
                    continue
                body = {"model": args.model, "messages": [{"role": "user", "content": Q}], "decoding": dec, "max_tokens": 400}
                if dec == "entropy_decoding":
                    body["max_new_tokens"] = 24
                status, resp, secs = post(f"http://127.0.0.1:{port}/v1/chat/completions", body, "smoke-local")
                tokens = (resp.get("usage") or {}).get("completion_tokens")
                rows.append((dec, status, secs, tokens, summarize(status, resp)))
                print(f"{dec:18} {status} {secs:6.1f}s  {rows[-1][4]}", flush=True)
        finally:
            proc.terminate()
    lines = [f"# Live smoke test ({args.model} via {args.base_url})", "",
             f"Generated {time.strftime('%Y-%m-%d %H:%M')} by parity/live_smoke.py.", "",
             "| slug | HTTP | seconds | completion tokens | answer (first 120 chars) |", "|---|---|---|---|---|"]
    for name, status, secs, tokens, text in rows:
        lines.append(f"| `{name}` | {status} | {secs:.1f} | {tokens} | {text.replace('|', '/')} |")
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text("\n".join(lines) + "\n")
    print(f"wrote {args.out}; server log in {home}/server.log")


if __name__ == "__main__":
    main()
