"""Render the parity table from parity/status.json + fixture counts.

Rewrites the block between `<!-- parity-table:start -->` and
`<!-- parity-table:end -->` in README.md and STATUS.md.

    python parity/parity_table.py           # rewrite in place
    python parity/parity_table.py --check   # exit 1 if out of date (CI)
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "parity" / "fixtures"
START, END = "<!-- parity-table:start -->", "<!-- parity-table:end -->"
GROUPS = {
    "server": "Server",
    "approach": "Approach",
    "local": "Local-inference approach",
    "plugin": "Plugin",
}
BADGE = {
    "parity": "✅ parity",
    "implemented": "🟡 implemented, not yet harness-verified",
    "partial": "🟠 partial",
    "emulated": "🟣 remote mode at parity; local mode emulated over API logprobs",
    "gap": "🔴 documented gap",
    "not started": "⬜ not started",
}


def fixture_count(dirs):
    total = 0
    for d in dirs:
        p = FIXTURES / d
        if p.is_dir():
            total += len(list(p.glob("*.json")))
    return total


def render():
    rows = json.loads((ROOT / "parity" / "status.json").read_text())["rows"]
    out = ["| Kind | Name | Status | Fixtures | Known deviations / notes |", "|---|---|---|---|---|"]
    for r in rows:
        notes = r["notes"].replace("|", "\\|") or "—"
        out.append(f"| {GROUPS[r['group']]} | `{r['name']}` | {BADGE[r['status']]} | {fixture_count(r['fixtures'])} | {notes} |")
    total = fixture_count([p.name for p in FIXTURES.iterdir() if p.is_dir()]) if FIXTURES.exists() else 0
    done = sum(1 for r in rows if r["status"] == "parity")
    out.append("")
    out.append(f"{done}/{len(rows)} rows at harness-verified parity, {total} fixtures in total.")
    return "\n".join(out)


def splice(text, table):
    pattern = re.compile(re.escape(START) + ".*?" + re.escape(END), re.S)
    block = f"{START}\n{table}\n{END}"
    if not pattern.search(text):
        raise SystemExit("parity-table markers not found")
    return pattern.sub(lambda _: block, text)


def main():
    check = "--check" in sys.argv
    table = render()
    stale = []
    for name in ("README.md", "STATUS.md"):
        path = ROOT / name
        old = path.read_text()
        new = splice(old, table)
        if new != old:
            stale.append(name)
            if not check:
                path.write_text(new)
    if check and stale:
        print("parity table out of date in: " + ", ".join(stale) + " (run python parity/parity_table.py)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
