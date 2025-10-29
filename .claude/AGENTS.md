# Agent Guidelines

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
