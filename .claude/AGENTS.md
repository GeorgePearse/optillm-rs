# Agent Guidelines

## Pre-Commit Hooks Setup

This project uses pre-commit hooks to automatically maintain code quality and documentation.

### Installation

```bash
# Install pre-commit framework (one time)
pip install pre-commit

# Set up hooks in this repository
pre-commit install

# Run all hooks manually
pre-commit run --all-files

# Skip hooks for emergency commits
SKIP=update-readme-structure,rustfmt,clippy git commit -m "emergency"
```

### Available Hooks

- **update-readme-structure** - Automatically updates project structure in README.md
- **trailing-whitespace** - Removes trailing whitespace
- **end-of-file-fixer** - Ensures files end with newline
- **check-yaml/toml/json** - Validates configuration files
- **detect-private-key** - Prevents committing secrets
- **rustfmt** - Formats Rust code
- **clippy** - Lints Rust code
- **markdownlint** - Lints and fixes markdown files

### How It Works

When you commit:
1. Pre-commit hooks automatically run
2. README.md is updated with current project structure
3. Code is formatted and checked for quality
4. If changes are made, re-stage and commit again
5. Commit succeeds when all hooks pass

**Note:** The README structure hook will update automatically - just re-run your commit!

---

## File Organization

When creating or editing documentation files:

- **scratch_pads/** - Use this directory for any temporary files, benchmarks, test results, or documentation with capitalized filenames (e.g., `BENCHMARK_RESULTS.md`, `TEST_OUTPUT.md`)
- Keep the root directory clean - only core project files at root level

### Naming Convention

- Capitalized filenames (e.g., `MY_FEATURE.md`) → Write to `scratch_pads/MY_FEATURE.md`
- Root-level documentation → Only `README.md`, `CLAUDE.md`, and `AGENTS.md` are permitted at root
- Standard project files → `Cargo.toml`, `mkdocs.yml`, `pyproject.toml`, etc. go in root

### Example

If you need to create a new documentation file called `NEW_ANALYSIS.md`:
```
Write to: scratch_pads/NEW_ANALYSIS.md
NOT: ./NEW_ANALYSIS.md
```

## Current Protected Root Files

- `README.md` - Main project documentation
- `CLAUDE.md` - Project-specific instructions
- `AGENTS.md` - This file
- `Cargo.toml` - Rust workspace configuration
- `mkdocs.yml` - Documentation configuration
- Other standard configuration files

All other capitalized `.md` files belong in `scratch_pads/`.
