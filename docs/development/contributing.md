# Contributing to optillm-rs

Thank you for your interest in contributing! This guide explains how to get involved.

## Code of Conduct

Be respectful and constructive. We welcome diverse perspectives and experiences.

## Getting Started

### 1. Fork the Repository

```bash
git clone https://github.com/YOUR_USERNAME/optillm-rs.git
cd optillm-rs
```

### 2. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
```

Use descriptive names:
- `feature/add-new-strategy`
- `fix/verification-issue`
- `docs/improve-setup-guide`
- `test/add-mars-tests`

## Development Workflow

### 1. Make Your Changes

```bash
# Edit files
# Add tests
# Update documentation
```

### 2. Run Tests

```bash
cargo test --all
```

### 3. Check Code Quality

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
```

### 4. Build Documentation

```bash
cargo doc --all --open
```

## Code Standards

### Rust Style

Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Examples
///
/// ```
/// let result = my_function(42);
/// assert_eq!(result, 84);
/// ```
///
/// # Errors
///
/// Returns an error if the input is invalid.
pub fn my_function(value: i32) -> Result<i32, Error> {
    // Implementation
}
```

### Documentation

- Every public item must have doc comments
- Include examples for complex functions
- Link to related items
- Explain error cases

### Testing

Every feature needs tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let result = my_function(42);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

## Types of Contributions

### 1. Bug Fixes

1. Create an issue describing the bug
2. Reference the issue in your PR
3. Include a test that reproduces the bug
4. Ensure the test passes with your fix

Example:
```
Fix: Update verification scoring calculation

The verification score was using incorrect formula.
Now uses exponential moving average as intended.

Fixes #123
```

### 2. Features

1. Discuss the feature in an issue first
2. Design and get feedback
3. Implement with tests
4. Update documentation
5. Submit PR

Example features:
- New optimization strategies
- Additional aggregation methods
- Provider support
- Configuration options

### 3. Documentation

- Improve existing docs
- Add examples
- Fix typos
- Clarify explanations
- Add diagrams

### 4. Tests

- Add unit tests
- Add integration tests
- Improve test coverage
- Add edge case tests

### 5. Performance

- Optimize hot paths
- Reduce memory usage
- Improve latency
- Include benchmarks

## Commit Messages

Use clear, descriptive messages:

```
feat: Add MCTS strategy implementation

- Implement UCB-based node selection
- Add dialogue state management
- Include comprehensive tests
- Update documentation

Closes #42
```

Format:
- Type: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`
- Short description (50 chars max)
- Blank line
- Detailed explanation (if needed)
- References to issues

## Pull Request Process

### 1. Create PR

```bash
git push origin feature/your-feature-name
# Create PR on GitHub
```

### 2. PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Test improvement

## Testing
- [ ] Unit tests added
- [ ] Integration tests added
- [ ] All tests pass

## Documentation
- [ ] Updated docs
- [ ] Added examples
- [ ] Updated README

## Checklist
- [ ] Code follows style guidelines
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No breaking changes
```

### 3. Review Process

- Expect feedback from maintainers
- Be responsive to comments
- Make requested changes
- Re-request review when ready

### 4. Merging

Once approved:
- Maintainer merges your PR
- Your changes are part of the project!

## Adding a New Optimizer

### Step 1: Create Structure

```bash
mkdir -p crates/my-optimizer/src
```

### Step 2: Implement Trait

```rust
use optillm_core::{Optimizer, Solution, ModelClient, Result};
use async_trait::async_trait;

pub struct MyOptimizer {
    // Configuration
}

#[async_trait]
impl Optimizer for MyOptimizer {
    async fn optimize(
        &self,
        query: &str,
        client: &dyn ModelClient,
    ) -> Result<Solution> {
        // Your implementation
    }

    fn name(&self) -> &str { "my-optimizer" }
    fn description(&self) -> &str { "Description" }
}
```

### Step 3: Add Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimization() {
        let optimizer = MyOptimizer::new();
        let result = optimizer.optimize("query", &mock_client).await;
        assert!(result.is_ok());
    }
}
```

### Step 4: Add to Workspace

Edit root `Cargo.toml`:
```toml
members = [
    "crates/core",
    "crates/mars",
    "crates/my-optimizer",  # Add this
]
```

### Step 5: Document

Create `crates/my-optimizer/README.md`:
```markdown
# my-optimizer

Description of the optimizer.

## Usage

```rust
let optimizer = MyOptimizer::new();
```

## Benchmarks

Performance metrics.
```

### Step 6: Submit PR

Include:
- Implementation with tests
- Documentation
- Example usage
- Benchmark results

## Documentation Contributions

### Building Documentation

```bash
# Build mkdocs
mkdocs build

# Serve locally
mkdocs serve  # Visit http://localhost:8000
```

### Structure

Docs follow this structure:
- `docs/index.md` - Homepage
- `docs/getting-started/` - Getting started
- `docs/architecture/` - Architecture
- `docs/mars/` - MARS docs
- `docs/strategies/` - Strategy docs
- `docs/development/` - Dev guides
- `docs/api/` - API reference

### Adding a Page

1. Create markdown file
2. Update `mkdocs.yml` navigation
3. Build and test: `mkdocs serve`
4. Submit PR

## Performance Contributions

### Benchmarking

```rust
fn main() {
    let start = Instant::now();
    // Your code
    println!("Time: {:?}", start.elapsed());
}
```

### Requirements for Performance PRs

1. Include benchmark results
2. Compare before/after
3. Explain optimization
4. Ensure no correctness loss
5. Document trade-offs

## Review Checklist

Before submitting, verify:

- [ ] Code follows Rust style guide
- [ ] All tests pass: `cargo test --all`
- [ ] No warnings: `cargo clippy`
- [ ] Code formatted: `cargo fmt`
- [ ] Documentation updated
- [ ] Examples work
- [ ] Commit messages clear
- [ ] No unrelated changes

## Questions?

- Open an issue
- Check discussions
- Ask in comments
- Read the docs

## Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Recognized in release notes
- Thanked in documentation

## License

By contributing, you agree your work will be licensed under MIT.

---

Thank you for contributing to optillm-rs! 🎉
