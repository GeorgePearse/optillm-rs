# Testing Guide

## Running Tests

### Run All Tests

```bash
cargo test --all
```

### Run Specific Crate Tests

```bash
cargo test -p optillm-core
cargo test -p optillm-mars
```

### Run Specific Test

```bash
cargo test test_name
```

### Show Output

```bash
cargo test -- --nocapture
```

### Run Tests in Release Mode

```bash
cargo test --release
```

## Test Organization

Tests are organized by module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_functionality() {
        // Test implementation
    }
}
```

## Types of Tests

### Unit Tests

```rust
#[test]
fn test_basic_function() {
    assert_eq!(add(2, 2), 4);
}
```

### Async Tests

```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_op().await;
    assert!(result.is_ok());
}
```

### Integration Tests

Place in `tests/` directory:

```bash
tests/
├── integration_test.rs
└── mars_integration.rs
```

## Test Coverage

```bash
# Using tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Best Practices

1. **Name tests clearly** - Describe what's being tested
2. **One assertion focus** - Test one thing per test
3. **Use descriptive names** - `test_mars_optimization_with_low_agents`
4. **Mock external dependencies** - Don't hit real APIs
5. **Test error cases** - Not just happy paths
6. **Keep tests fast** - Unit tests should be quick

## Example Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_network_registration() {
        let mut network = StrategyNetwork::new();

        let id = network.register_strategy(
            "agent-1".to_string(),
            "Test strategy".to_string(),
            "Test technique".to_string(),
        );

        assert!(!id.is_empty());
        assert_eq!(network.count_strategies(), 1);
    }

    #[tokio::test]
    async fn test_mars_coordinator() {
        let config = MarsConfig::default();
        let coordinator = MarsCoordinator::new(config);

        // Test with mock client
        let result = coordinator.optimize("test query", &mock_client).await;
        assert!(result.is_ok());
    }
}
```

## Continuous Integration

Tests run automatically on:
- Every commit
- Every PR
- Main branch updates

Check `.github/workflows/` for CI configuration.

---

For more details, see [Contributing Guide](contributing.md)
