# Agent Guidelines

## Prek Hooks Setup

This project uses prek hooks to automatically maintain code quality and documentation.

### Installation

```bash
# Install prek framework (one time)
pip install prek

# Set up hooks in this repository
prek install

# Run all hooks manually
prek run --all-files

# View hook status
prek status

# Skip specific hooks for emergency commits
prek run --skip update-readme-structure,rustfmt git commit -m "emergency"
```

### Available Hooks

- **update-readme-structure** - Automatically updates project structure in README.md
- **rustfmt** - Formats Rust code with rustfmt
- **clippy** - Lints Rust code and prevents warnings
- **trailing-whitespace** - Removes trailing whitespace
- **check-yaml** - Validates YAML syntax
- **check-toml** - Validates TOML syntax
- **check-json** - Validates JSON syntax
- **detect-private-key** - Prevents committing secrets
- **markdown-lint** - Lints and fixes markdown files

### How It Works

When you commit with prek:
1. Prek hooks automatically run before commit
2. README.md is updated with current project structure
3. Rust code is formatted and checked for quality
4. Configuration files are validated
5. If changes are made, re-stage and commit again
6. Commit succeeds when all hooks pass

**Note:** The README structure hook will update automatically - just re-run your commit!

### Configuration

Prek configuration is in `.prek.yaml`. Edit this file to:
- Add or remove hooks
- Modify hook behavior
- Configure parallel execution
- Set up different stages (commit, push, etc.)

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
