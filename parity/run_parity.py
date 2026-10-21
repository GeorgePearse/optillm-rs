"""Behavioural parity harness: upstream optillm vs optillm-rs.

For every fixture, both servers are pointed at their own instance of the
same deterministic mock backend (parity/mock_backend.py) with the same
script. The fixture's request(s) are sent to both servers and the harness
asserts that
  1. the sequence of LLM calls each server made (path, auth header and full
     JSON body: messages, system prompts, n, temperature, max_tokens, ...)
     is identical, and
  2. the HTTP responses (status + body) are identical, after normalising the
     wall-clock ids / timestamps both servers generate.

Fixtures live in parity/fixtures/<approach-or-plugin>/<case>.json:
    {
      "description": "...",
      "server_args": ["--best-of-n", "4"],  # extra CLI args for both servers
      "env": {"OPTILLM_N": "2"},            # extra env vars for both servers
      "seed": 0,                            # reseed `random` per request
      "requests": [{"method": "POST", "path": "/v1/chat/completions",
                    "headers": {...}, "body": {...}}],
      "mock": {"rules": [...], "models": [...]},
      "order": "exact" | "any",             # "any": concurrent calls upstream
      "xfail": "documented deviation"       # expected to differ
    }

Usage:
    python parity/run_parity.py --upstream-python ~/optillm-upstream/venv/bin/python \
        --rust-bin target/debug/optillm-proxy [--filter bon] [--report out.json]
"""
import argparse
import difflib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
API_KEY = "sk-parity-test-key"


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def http(method, url, body=None, headers=None, timeout=300):
    data = None
    hdrs = dict(headers or {})
    if body is not None:
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
        hdrs.setdefault("Content-Type", "application/json")
    req = urllib.request.Request(url, data=data, method=method, headers=hdrs)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status, resp.headers.get("Content-Type", ""), resp.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.headers.get("Content-Type", ""), e.read().decode()


def wait_for(url, proc, name, timeout=120):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(f"{name} exited early with code {proc.returncode}")
        try:
            status, _, _ = http("GET", url, timeout=2)
            if status in (200, 401, 404):
                return
        except Exception:
            pass
        time.sleep(0.2)
    raise RuntimeError(f"{name} did not come up at {url}")


class Stack:
    """A mock backend plus one server under test."""

    def __init__(self, name, server_cmd, env, workdir, logdir):
        self.name = name
        self.mock_port = free_port()
        self.port = free_port()
        self.workdir = workdir
        self.mock_log = open(logdir / f"{name}-mock.log", "w")
        self.server_log = open(logdir / f"{name}-server.log", "w")
        self.mock = subprocess.Popen(
            [sys.executable, str(HERE / "mock_backend.py"), "--port", str(self.mock_port)],
            stdout=self.mock_log,
            stderr=subprocess.STDOUT,
        )
        wait_for(f"http://127.0.0.1:{self.mock_port}/__mock/calls", self.mock, f"{name} mock")
        cmd = server_cmd + [
            "--port",
            str(self.port),
            "--base-url",
            f"http://127.0.0.1:{self.mock_port}/v1",
        ]
        self.server = subprocess.Popen(
            cmd, cwd=workdir, env=env, stdout=self.server_log, stderr=subprocess.STDOUT
        )
        wait_for(f"http://127.0.0.1:{self.port}/health", self.server, f"{name} server")

    def script(self, mock):
        http("POST", f"http://127.0.0.1:{self.mock_port}/__mock/script", mock)

    def calls(self):
        _, _, body = http("GET", f"http://127.0.0.1:{self.mock_port}/__mock/calls")
        return json.loads(body)["calls"]

    def request(self, r):
        headers = dict(r.get("headers", {}))
        body = r.get("body")
        if "raw_body" in r:
            body = r["raw_body"].encode()
        return http(r.get("method", "POST"), f"http://127.0.0.1:{self.port}{r.get('path', '/v1/chat/completions')}", body, headers)

    def close(self):
        for p in (self.server, self.mock):
            p.terminate()
            try:
                p.wait(timeout=10)
            except subprocess.TimeoutExpired:
                p.kill()
        self.mock_log.close()
        self.server_log.close()


TS_ID = re.compile(r"chatcmpl-\d{12,}(-\d+)?")


def normalize_value(v):
    """Replace wall-clock ids / timestamps produced by the servers themselves."""
    if isinstance(v, dict):
        out = {}
        for k, x in v.items():
            if k == "created" and isinstance(x, int) and x > 1_000_000_000:
                out[k] = "<created>"
            else:
                out[k] = normalize_value(x)
        return out
    if isinstance(v, list):
        return [normalize_value(x) for x in v]
    if isinstance(v, str):
        return TS_ID.sub("chatcmpl-<ts>", v)
    return v


TS_CREATED = re.compile(r'"created": ?1\d{9}\b')


def normalize_text(text):
    return TS_CREATED.sub('"created": <created>', TS_ID.sub("chatcmpl-<ts>", text))


def normalize_body(content_type, text):
    """(kind, structural value, normalised raw text)."""
    if "event-stream" in content_type:
        return "SSE", normalize_text(text), normalize_text(text)
    try:
        return "JSON", normalize_value(json.loads(text)), normalize_text(text)
    except json.JSONDecodeError:
        return "TEXT", text, text


def pretty(v):
    if isinstance(v, str):
        return v
    return json.dumps(v, indent=1, sort_keys=True, ensure_ascii=False)


def diff(a, b, label):
    lines = difflib.unified_diff(
        pretty(a).splitlines(), pretty(b).splitlines(), f"upstream {label}", f"rust {label}", lineterm="", n=2
    )
    out = list(lines)
    return "\n".join(out[:120]) + ("\n..." if len(out) > 120 else "")


# Tracebacks of z3 solver code name the file that ran exec(): upstream's
# site-packages z3_solver.py vs optillm-rs's z3_runner.py temp file.
EXEC_FRAME = re.compile(r'File \\"[^"\\]+\\", line \d+, in execute_code_in_process')


def canonical_call(c):
    return EXEC_FRAME.sub('File \\"<z3 exec frame>\\", line N, in execute_code_in_process', json.dumps(c, sort_keys=True))


SHOW = False


def run_fixture(fx, up, rs):
    problems = []
    for stack in (up, rs):
        stack.script(fx.get("mock", {"rules": []}))
    requests = fx.get("requests") or [fx["request"]]
    for i, r in enumerate(requests):
        ures = up.request(r)
        rres = rs.request(r)
        if SHOW:
            print(f"--- {fx['_name']} request {i}\n  upstream [{ures[0]}]: {ures[2][:600]}\n  rust     [{rres[0]}]: {rres[2][:600]}")
        ukind, ubody, uraw = normalize_body(ures[1], ures[2])
        rkind, rbody, rraw = normalize_body(rres[1], rres[2])
        if ures[0] != rres[0]:
            problems.append(f"request {i}: status upstream={ures[0]} rust={rres[0]}\n  upstream: {uraw[:400]}\n  rust: {rraw[:400]}")
        if ukind != rkind or ubody != rbody:
            problems.append(f"request {i}: response body differs\n" + diff(ubody, rbody, "response"))
        elif uraw != rraw:
            # Same structure but different bytes (key order, escaping, separators).
            problems.append(f"request {i}: response bytes differ\n" + diff(uraw, rraw, "raw"))
    ucalls, rcalls = up.calls(), rs.calls()
    if fx.get("order", "exact") == "any":
        ucalls = sorted(ucalls, key=canonical_call)
        rcalls = sorted(rcalls, key=canonical_call)
    if [canonical_call(c) for c in ucalls] != [canonical_call(c) for c in rcalls]:
        problems.append(
            f"LLM call sequence differs (upstream {len(ucalls)} calls, rust {len(rcalls)} calls)\n"
            + diff(ucalls, rcalls, "calls")
        )
    return problems, len(ucalls)


def load_fixtures(root, flt):
    fixtures = []
    for path in sorted(root.glob("*/*.json")):
        rel = f"{path.parent.name}/{path.stem}"
        if flt and not re.search(flt, rel):
            continue
        fx = json.loads(path.read_text())
        fx["_name"] = rel
        fixtures.append(fx)
    return fixtures


def group_key(fx):
    return json.dumps([fx.get("server_args", []), fx.get("env", {}), fx.get("seed")], sort_keys=True)


UPSTREAM_PYTHON = None


def base_env(extra, seed, rust):
    env = {
        "PATH": os.environ.get("PATH", ""),
        "HOME": os.environ.get("PARITY_HOME", tempfile.gettempdir()),
        "OPENAI_API_KEY": API_KEY,
        "LANG": "C.UTF-8",
        "RUST_LOG": os.environ.get("RUST_LOG", "warn"),
        "HF_HUB_OFFLINE": "1",
        "TRANSFORMERS_OFFLINE": "1",
    }
    if rust:
        # The z3 approach executes solver code with CPython + z3 + sympy.
        env["OPTILLM_Z3_PYTHON"] = UPSTREAM_PYTHON
    else:
        env["PYTHONPATH"] = str(HERE / "upstream_hooks")
        env["PYTHONUNBUFFERED"] = "1"
    if seed is not None:
        env["PARITY_SEED"] = str(seed)
        env["OPTILLM_RANDOM_SEED"] = str(seed)
    env.update(extra)
    return env


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--upstream-python", required=True, help="python of the venv with optillm==0.4.0")
    ap.add_argument("--rust-bin", required=True, help="path to the optillm-proxy binary")
    ap.add_argument("--fixtures", default=str(HERE / "fixtures"))
    ap.add_argument("--filter", default=None, help="regex on <dir>/<case>")
    ap.add_argument("--report", default=None, help="write a JSON report here")
    ap.add_argument("--logs", default=None, help="directory for server / mock logs")
    ap.add_argument("--show", action="store_true", help="print both raw responses")
    args = ap.parse_args()
    global SHOW, UPSTREAM_PYTHON
    SHOW = args.show
    UPSTREAM_PYTHON = args.upstream_python

    fixtures = load_fixtures(Path(args.fixtures), args.filter)
    if not fixtures:
        print("no fixtures selected")
        return 1
    logdir = Path(args.logs or tempfile.mkdtemp(prefix="parity-logs-"))
    logdir.mkdir(parents=True, exist_ok=True)
    upstream_bin = str(Path(args.upstream_python).parent / "optillm")
    rust_bin = str(Path(args.rust_bin).resolve())

    groups = {}
    for fx in fixtures:
        groups.setdefault(group_key(fx), []).append(fx)

    results = []
    for gi, (key, group) in enumerate(groups.items()):
        server_args, extra_env, seed = json.loads(key)
        server_args = [a.replace("{PARITY_DIR}", str(HERE)) for a in server_args]
        workdir = Path(tempfile.mkdtemp(prefix="parity-cwd-"))
        up = rs = None
        try:
            up = Stack(
                f"g{gi}-upstream",
                [upstream_bin, "--log", "warning"] + server_args,
                base_env(extra_env, seed, rust=False),
                workdir,
                logdir,
            )
            rs = Stack(f"g{gi}-rust", [rust_bin] + server_args, base_env(extra_env, seed, rust=True), workdir, logdir)
            for fx in group:
                t0 = time.time()
                try:
                    problems, ncalls = run_fixture(fx, up, rs)
                except Exception as e:  # harness-level failure
                    problems, ncalls = [f"harness error: {e!r}"], 0
                xfail = fx.get("xfail")
                if problems and xfail:
                    status = "XFAIL"
                elif not problems and xfail:
                    status = "XPASS"
                else:
                    status = "PASS" if not problems else "FAIL"
                results.append(
                    {"fixture": fx["_name"], "status": status, "calls": ncalls, "seconds": round(time.time() - t0, 2), "problems": problems, "xfail": xfail}
                )
                print(f"{status:5} {fx['_name']}  ({ncalls} LLM calls, {time.time() - t0:.1f}s)")
                if status in ("FAIL", "XPASS"):
                    for p in problems:
                        print("      " + p.replace("\n", "\n      "))
                    if status == "XPASS":
                        print(f"      fixture is marked xfail ({xfail}) but now matches; remove the xfail")
        finally:
            for s in (up, rs):
                if s:
                    s.close()
            shutil.rmtree(workdir, ignore_errors=True)

    counts = {}
    for r in results:
        counts[r["status"]] = counts.get(r["status"], 0) + 1
    print("\nsummary:", ", ".join(f"{k}={v}" for k, v in sorted(counts.items())), f"(logs in {logdir})")
    if args.report:
        Path(args.report).write_text(json.dumps({"results": results, "counts": counts}, indent=1))
    return 1 if counts.get("FAIL") or counts.get("XPASS") else 0


if __name__ == "__main__":
    sys.exit(main())
