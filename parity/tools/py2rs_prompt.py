"""Convert a Python string / f-string literal from upstream into a Rust literal.

Prompts must be byte-identical to upstream. Hand-copying multi-line prompts
loses trailing spaces (and the trailing-whitespace hook strips them from raw
strings), so this tool reads the literal through `ast` and emits a
`concat!(...)` with one quoted line per source line, which survives
formatting hooks. f-strings become a `format!` template with positional
placeholders `{p0}`, `{p1}`, ... and a comment naming each original
expression.

Usage:
    python parity/tools/py2rs_prompt.py --all <file.py> [min_len]  # every long literal
    python parity/tools/py2rs_prompt.py <file.py> <line>   # literal starting on <line>
    python parity/tools/py2rs_prompt.py <file.py> <NAME>   # module/class constant NAME
"""
import ast
import sys


def rust_escape(s: str) -> str:
    out = []
    for ch in s:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif ch == "\r":
            out.append("\\r")
        elif ord(ch) < 0x20 or ord(ch) == 0x7F:
            out.append("\\u{%x}" % ord(ch))
        else:
            out.append(ch)
    return "".join(out)


def concat_lines(s: str) -> str:
    parts = s.split("\n")
    pieces = []
    for i, p in enumerate(parts):
        last = i == len(parts) - 1
        text = p + ("" if last else "\n")
        if text:
            pieces.append('"' + rust_escape(text) + '"')
    if not pieces:
        return '""'
    if len(pieces) == 1:
        return pieces[0]
    return "concat!(\n    " + ",\n    ".join(pieces) + ",\n)"


def convert(node, source: str) -> str:
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return concat_lines(node.value)
    if isinstance(node, ast.JoinedStr):
        template = []
        exprs = []
        for v in node.values:
            if isinstance(v, ast.Constant):
                template.append(v.value.replace("{", "{{").replace("}", "}}"))
            else:
                spec = ""
                if v.format_spec is not None:
                    spec = ":" + "".join(
                        c.value for c in v.format_spec.values if isinstance(c, ast.Constant)
                    )
                conv = {-1: "", 115: "!s", 114: "!r", 97: "!a"}[v.conversion]
                template.append("{p%d%s}" % (len(exprs), spec))
                exprs.append(ast.get_source_segment(source, v.value) + conv)
        body = concat_lines("".join(template))
        args = "\n".join("    // p%d = %s" % (i, e) for i, e in enumerate(exprs))
        return "format!(\n" + body + ",\n" + args + "\n)"
    raise SystemExit(f"unsupported node {type(node).__name__}")


def dump_all(path, min_len):
    """Print every string / f-string literal of at least min_len characters."""
    source = open(path).read()
    tree = ast.parse(source)
    seen = set()
    nodes = []
    for node in ast.walk(tree):
        if isinstance(node, ast.JoinedStr):
            for v in node.values:
                seen.add(id(v))
            nodes.append(node)
    for node in ast.walk(tree):
        if isinstance(node, ast.Constant) and isinstance(node.value, str) and id(node) not in seen:
            nodes.append(node)
    for node in sorted(nodes, key=lambda n: (n.lineno, n.col_offset)):
        text = ast.unparse(node)
        if len(text) >= min_len:
            print(f"=== line {node.lineno}")
            print(convert(node, source))


def main():
    if sys.argv[1] == "--all":
        dump_all(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 60)
        return
    path, target = sys.argv[1], sys.argv[2]
    source = open(path).read()
    tree = ast.parse(source)
    found = None
    candidates = []
    for node in ast.walk(tree):
        if target.isdigit():
            if isinstance(node, (ast.Constant, ast.JoinedStr)) and node.lineno == int(target):
                if isinstance(node, ast.JoinedStr) or isinstance(node.value, str):
                    candidates.append(node)
        elif isinstance(node, ast.Assign):
            for t in node.targets:
                if isinstance(t, ast.Name) and t.id == target:
                    found = node.value
    if candidates:
        # Several literals can start on one line ("content": f"""..."""): take
        # the f-string if there is one, else the longest string.
        found = max(candidates, key=lambda n: (isinstance(n, ast.JoinedStr), len(ast.unparse(n))))
    if found is None:
        raise SystemExit("literal not found")
    print(convert(found, source))


if __name__ == "__main__":
    main()
