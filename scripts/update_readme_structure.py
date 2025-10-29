#!/usr/bin/env python3
"""
Pre-commit hook to automatically update the project structure in README.md
Run with: python3 scripts/update_readme_structure.py
"""

import re
import sys
from pathlib import Path

PROJECT_TREE = """optillm-rs/
├── .claude/
│   ├── AGENTS.md
│   ├── CLAUDE.md
│   └── settings.local.json
├── .github/
├── .gitignore
├── Cargo.lock
├── Cargo.toml
├── README.md
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── client.rs
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       ├── optimizer.rs
│   │       └── solution.rs
│   │
│   └── mars/
│       ├── Cargo.toml
│       ├── README.md
│       └── src/
│           ├── config.rs
│           ├── core/
│           ├── core_compat.rs
│           ├── error.rs
│           ├── lib.rs
│           ├── providers/
│           ├── strategies/
│           └── types.rs
├── docs/
├── examples/
├── mkdocs.yml
├── scratch_pads/
│   ├── CODING_LLM_BENCHMARKS.md
│   ├── COMPREHENSIVE_STRATEGY_BENCHMARK_RESULTS.md
│   ├── MODAL_BENCHMARK_SETUP.md
│   ├── TINYLLAMA_STRATEGY_TEST_RESULTS.md
│   └── ULTRA_TINY_MODELS.md
└── modal_benchmark.py"""


def update_readme():
    """Update README.md with current project structure."""
    readme_path = Path("README.md")

    if not readme_path.exists():
        print("❌ README.md not found")
        return False

    content = readme_path.read_text()

    # Pattern to match the project structure code block
    pattern = r"(## Project Structure\n\n```\n).*?(\n```)"

    replacement = f"\\1{PROJECT_TREE}\\2"

    new_content = re.sub(pattern, replacement, content, flags=re.DOTALL)

    if new_content != content:
        readme_path.write_text(new_content)
        print("✅ README.md updated with current project structure")
        return True
    else:
        print("✓ README.md is already up to date")
        return False


if __name__ == "__main__":
    try:
        updated = update_readme()
        sys.exit(0)  # Always exit 0 to not block commits
    except Exception as e:
        print(f"⚠️ Warning: Could not update README: {e}")
        sys.exit(0)  # Don't block commits on error
