"""Deterministic, recording mock of an OpenAI-compatible backend.

Every request is recorded. Responses come from a script of rules uploaded
per fixture; a request that matches no rule gets a reply derived from a
hash of the request itself, so two servers that send identical requests get
identical replies regardless of ordering or concurrency.

Control endpoints (not part of the OpenAI surface):
    POST /__mock/script   {"rules": [...], "models": [...]}  -> resets calls
    GET  /__mock/calls    -> {"calls": [...]}

Rule shape:
    {
      "match": {                     # every given key must match (all optional)
        "contains": "text" | [..],   # substring(s) of json.dumps(messages)
        "last_contains": "text",     # substring of the last message's content
        "system_contains": "text",   # substring of the first system message
        "not_contains": "text" | [..],
        "n": 3, "temperature": 0.1, "max_tokens": 512, "model": "m",
        "has": ["logprobs"],         # keys that must be present in the body
        "lacks": ["n"]               # keys that must be absent from the body
      },
      "response": {...}              # or "responses": [{...}, ...] consumed in order
    }

Serialised mode ("serialize_ms": 150 next to "rules"): concurrent requests are
held until no request has arrived (or been released) for serialize_ms, then
released one at a time in canonical-body order. Upstream approaches that fan
out with asyncio / threads (rstar, mars, cepo, ...) then resume in the same
order in both servers, which makes their seeded random draws comparable.

Response spec:
    {"content": "x"} | {"contents": ["a", "b"]}     # cycled over n choices
    {"content": null}                               # message.content = null
    "finish_reason": "stop" | "length" | [..]       # per choice
    "usage": {...} | null                           # override / remove usage
    "status": 400, "body": {...}                    # error response
    "raw": {...}                                    # exact response body
    "tool_calls": [...], "reasoning_content": "..."
    "logprobs": [ {...choice logprobs...} ]         # per choice, cycled
    "extra": {...}                                  # merged into the top level

Usage: python parity/mock_backend.py --port 18001
"""
import argparse
import hashlib
import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LOCK = threading.Lock()
STATE = {"rules": [], "counters": {}, "calls": [], "models": None, "serialize_ms": None, "pages": {}}
GATE = threading.Condition()
PENDING = []  # [canonical_body, released_flag_list]
LAST_EVENT = [0.0]


def gate_wait(body):
    """Block until the scheduler releases this request (serialised mode)."""
    entry = [canonical(body), [False]]
    with GATE:
        PENDING.append(entry)
        LAST_EVENT[0] = time.time()
        GATE.notify_all()
        while not entry[1][0]:
            GATE.wait(0.01)


def scheduler():
    while True:
        time.sleep(0.005)
        quiet = STATE.get("serialize_ms")
        if not quiet:
            continue
        with GATE:
            if PENDING and time.time() - LAST_EVENT[0] >= quiet / 1000.0:
                PENDING.sort(key=lambda e: e[0])
                entry = PENDING.pop(0)
                entry[1][0] = True
                LAST_EVENT[0] = time.time()
                GATE.notify_all()


def canonical(obj):
    return json.dumps(obj, sort_keys=True, ensure_ascii=False)


def digest(obj, n=12):
    return hashlib.sha256(canonical(obj).encode()).hexdigest()[:n]


def as_list(v):
    if v is None:
        return []
    return v if isinstance(v, list) else [v]


def message_text(m):
    c = m.get("content")
    if isinstance(c, list):
        return " ".join(str(p.get("text", "")) for p in c if isinstance(p, dict))
    return "" if c is None else str(c)


def matches(rule_match, body):
    messages = body.get("messages") or []
    blob = json.dumps(messages, ensure_ascii=False)
    for s in as_list(rule_match.get("contains")):
        if s not in blob:
            return False
    for s in as_list(rule_match.get("not_contains")):
        if s in blob:
            return False
    if "last_contains" in rule_match:
        last = message_text(messages[-1]) if messages else ""
        if rule_match["last_contains"] not in last:
            return False
    if "system_contains" in rule_match:
        systems = [message_text(m) for m in messages if m.get("role") == "system"]
        if not systems or rule_match["system_contains"] not in systems[0]:
            return False
    for key in ("n", "temperature", "max_tokens", "model", "top_p", "logprobs", "top_logprobs"):
        if key in rule_match and body.get(key) != rule_match[key]:
            return False
    for key in as_list(rule_match.get("has")):
        if key not in body:
            return False
    for key in as_list(rule_match.get("lacks")):
        if key in body:
            return False
    return True


def tokens_for(text):
    if text is None:
        return 0
    return max(1, len(text) // 4)


def build_completion(spec, body):
    if "raw" in spec:
        return 200, spec["raw"]
    if "status" in spec:
        return spec["status"], spec.get("body", {"error": {"message": "mock error"}})
    n = body.get("n") or 1
    if not isinstance(n, int) or n < 1:
        n = 1
    if "contents" in spec:
        contents = spec["contents"]
    elif "content" in spec:
        contents = [spec["content"]]
    else:
        contents = ["Mock response " + digest(body, 8)]
    finish = as_list(spec.get("finish_reason", "stop"))
    logprobs = spec.get("logprobs")
    choices = []
    for i in range(n):
        content = contents[i % len(contents)]
        message = {"role": "assistant", "content": content}
        if "tool_calls" in spec:
            message["tool_calls"] = spec["tool_calls"]
        if "reasoning_content" in spec:
            message["reasoning_content"] = spec["reasoning_content"]
        choice = {"index": i, "message": message, "finish_reason": finish[i % len(finish)]}
        if logprobs is not None:
            choice["logprobs"] = logprobs[i % len(logprobs)]
        choices.append(choice)
    completion_tokens = sum(tokens_for(c["message"]["content"]) for c in choices)
    prompt_tokens = max(1, len(json.dumps(body.get("messages") or [])) // 4)
    resp = {
        "id": "chatcmpl-mock-" + digest(body),
        "object": "chat.completion",
        "created": 42,
        "model": body.get("model"),
        "choices": choices,
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
        },
    }
    if "usage" in spec:
        if spec["usage"] is None:
            del resp["usage"]
        else:
            resp["usage"] = spec["usage"]
    resp.update(spec.get("extra", {}))
    return 200, resp


def respond_chat(body):
    with LOCK:
        for idx, rule in enumerate(STATE["rules"]):
            if matches(rule.get("match", {}), body):
                count = STATE["counters"].get(idx, 0)
                STATE["counters"][idx] = count + 1
                if "responses" in rule:
                    seq = rule["responses"]
                    spec = seq[min(count, len(seq) - 1)]
                else:
                    spec = rule.get("response", {})
                break
        else:
            spec = {}
    return build_completion(spec, body)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def _send(self, status, payload):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _body(self):
        length = int(self.headers.get("Content-Length") or 0)
        raw = self.rfile.read(length) if length else b""
        try:
            return json.loads(raw or b"{}")
        except json.JSONDecodeError:
            return {"__raw__": raw.decode("utf-8", "replace")}

    def _record(self, body):
        with LOCK:
            STATE["calls"].append(
                {
                    "method": self.command,
                    "path": self.path,
                    "authorization": self.headers.get("Authorization") or self.headers.get("api-key"),
                    "body": body,
                }
            )

    def do_GET(self):
        if self.path == "/__mock/calls":
            with LOCK:
                return self._send(200, {"calls": list(STATE["calls"])})
        self._record(None)
        path = self.path.split("?")[0]
        if path.startswith("/__page/"):
            page = STATE["pages"].get(path[len("/__page/"):])
            if page is None:
                return self._send(404, {"error": "no such page"})
            data = page.get("html", "").encode()
            self.send_response(page.get("status", 200))
            self.send_header("Content-Type", page.get("content_type", "text/html; charset=utf-8"))
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        if "/models/" in path:
            model_id = path.split("/models/", 1)[1]
            for m in STATE["models"] or []:
                if m.get("id") == model_id:
                    return self._send(200, m)
            return self._send(404, {"error": {"message": f"The model '{model_id}' does not exist", "type": "invalid_request_error", "code": "model_not_found"}})
        if self.path.split("?")[0].endswith("/models"):
            models = STATE["models"]
            if models is None:
                models = [{"id": "mock-model", "object": "model", "created": 42, "owned_by": "mock"}]
            return self._send(200, {"object": "list", "data": models})
        self._send(404, {"error": {"message": "not found"}})

    def do_POST(self):
        body = self._body()
        if self.path == "/__mock/script":
            with LOCK:
                STATE["rules"] = body.get("rules", [])
                STATE["models"] = body.get("models")
                STATE["serialize_ms"] = body.get("serialize_ms")
                STATE["pages"] = body.get("pages") or {}
                STATE["counters"] = {}
                STATE["calls"] = []
            return self._send(200, {"ok": True})
        self._record(body)
        if self.path.split("?")[0].endswith("/chat/completions"):
            if STATE.get("serialize_ms"):
                gate_wait(body)
            status, payload = respond_chat(body)
            return self._send(status, payload)
        self._send(404, {"error": {"message": "not found"}})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args()
    threading.Thread(target=scheduler, daemon=True).start()
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    server.daemon_threads = True
    server.serve_forever()


if __name__ == "__main__":
    main()
