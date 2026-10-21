#!/usr/bin/env python3
"""Fail if any given file contains a private key or a real-looking API key."""
import re
import sys

PATTERNS = [
    re.compile(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----"),
    # OpenAI / OpenRouter / Anthropic / DeepSeek style keys.
    re.compile(r"\bsk-(?:or-v1-|ant-|proj-)?[A-Za-z0-9_-]{32,}"),
    re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
]

bad = []
for path in sys.argv[1:]:
    try:
        text = open(path, encoding="utf-8", errors="ignore").read()
    except OSError:
        continue
    for pat in PATTERNS:
        if pat.search(text):
            bad.append(path)
            break

for path in bad:
    print(f"possible secret in {path}")
sys.exit(1 if bad else 0)
