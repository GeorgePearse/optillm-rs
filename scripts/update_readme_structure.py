#!/usr/bin/env python3
"""
Pre-commit hook to automatically update the project structure in README.md
Dynamically scans the directory tree and generates structure from filesystem.
Run with: python3 scripts/update_readme_structure.py
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple

# Directories and files to exclude from tree
EXCLUDE_DIRS = {
    ".git",
    ".venv",
    "venv",
    "node_modules",
    "target",
    "__pycache__",
    ".pytest_cache",
    ".vscode",
    ".idea",
    "dist",
    "build",
    "*.egg-info",
}

EXCLUDE_FILES = {
    ".DS_Store",
    ".gitkeep",
    "*.pyc",
    ".lock",
}

# Maximum depth to show (0 = root only, -1 = unlimited)
MAX_DEPTH = 4


def should_include(path: Path, depth: int) -> bool:
    """Check if path should be included in tree."""
    if depth > MAX_DEPTH:
        return False

    name = path.name

    # Exclude specific directories and files
    if name in EXCLUDE_DIRS:
        return False

    if any(name.endswith(pattern.replace("*", "")) for pattern in EXCLUDE_FILES):
        return False

    return True


def generate_tree(root: Path, prefix: str = "", is_last: bool = True, depth: int = 0) -> str:
    """
    Generate a tree structure string from filesystem.

    Args:
        root: Root path to generate tree from
        prefix: Current line prefix (for indentation)
        is_last: Whether this is the last item in parent
        depth: Current depth in tree

    Returns:
        Tree structure as string
    """
    lines = []

    # Add root directory name at depth 0
    if depth == 0:
        # Use the absolute path name, or fallback to project name detection
        root_name = root.resolve().name or "optillm-rs"
        lines.append(f"{root_name}/")

    if depth >= MAX_DEPTH:
        return "\n".join(lines)

    try:
        # Get all items and sort them
        items = sorted([p for p in root.iterdir() if should_include(p, depth + 1)])
    except PermissionError:
        return "\n".join(lines)

    for i, item in enumerate(items):
        is_last_item = i == len(items) - 1

        # Determine tree characters
        connector = "└── " if is_last_item else "├── "
        next_prefix = prefix + ("    " if is_last_item else "│   ")

        # Add item to tree
        if item.is_dir():
            lines.append(f"{prefix}{connector}{item.name}/")

            # Recursively add subdirectories (limited depth)
            if depth + 1 < MAX_DEPTH:
                try:
                    sub_items = sorted([p for p in item.iterdir() if should_include(p, depth + 2)])
                    for j, sub_item in enumerate(sub_items):
                        is_last_sub = j == len(sub_items) - 1
                        sub_connector = "└── " if is_last_sub else "├── "
                        sub_next_prefix = next_prefix + ("    " if is_last_sub else "│   ")

                        if sub_item.is_dir():
                            lines.append(f"{next_prefix}{sub_connector}{sub_item.name}/")
                        else:
                            lines.append(f"{next_prefix}{sub_connector}{sub_item.name}")
                except PermissionError:
                    pass
        else:
            lines.append(f"{prefix}{connector}{item.name}")

    return "\n".join(lines)


def update_readme():
    """Update README.md with current project structure."""
    readme_path = Path("README.md")

    if not readme_path.exists():
        print("❌ README.md not found")
        return False

    # Generate tree from current filesystem
    root = Path(".")
    project_tree = generate_tree(root)

    content = readme_path.read_text()

    # Pattern to match the project structure code block
    pattern = r"(## Project Structure\n\n```\n).*?(\n```)"

    replacement = f"\\1{project_tree}\\2"

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
