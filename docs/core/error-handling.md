# Error Handling

OptimLLM uses the `thiserror` crate for comprehensive, ergonomic error handling.

## OptillmError Enum

```rust
pub enum OptillmError {
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Client error: {0}")]
    ClientError(String),

    #[error("Parsing error: {0}")]
    ParsingError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    // ... more variants specific to MARS
}
```

## Result Type Alias

The standard return type for all optillm operations:

```rust
pub type Result<T> = std::result::Result<T, OptillmError>;
```

## Error Creation

### Returning Errors

```rust
pub fn validate_config(config: &MyConfig) -> Result<()> {
    if config.temperature > 2.0 {
        return Err(OptillmError::InvalidConfiguration(
            "Temperature must be <= 2.0".to_string()
        ));
    }
    Ok(())
}
```

### Using the `?` Operator

```rust
pub async fn my_optimizer(
    query: &str,
    client: &dyn ModelClient,
) -> Result<Solution> {
    // Errors automatically convert and propagate
    let result = client.stream(&prompt);

    Ok(solution)
}
```

### Converting from Other Errors

```rust
// From serde_json errors
let solution: Solution = serde_json::from_str(json)
    .map_err(|e| OptillmError::ParsingError(e.to_string()))?;

// From I/O errors
let content = std::fs::read_to_string("config.toml")
    .map_err(|e| OptillmError::ClientError(e.to_string()))?;
```

## Error Handling in Async Code

### With `?` operator

```rust
async fn process(client: &dyn ModelClient) -> Result<String> {
    let stream = client.stream(&prompt);
    let result = stream.next().await?; // Propagates error

    Ok(result)
}
```

### With Match

```rust
async fn process(client: &dyn ModelClient) -> Result<String> {
    match client.stream(&prompt).next().await {
        Some(Ok(event)) => {
            // Handle event
            Ok(String::new())
        }
        Some(Err(e)) => Err(e),
        None => Err(OptillmError::ClientError(
            "Stream ended unexpectedly".to_string()
        )),
    }
}
```

### With `map_err`

```rust
async fn process(client: &dyn ModelClient) -> Result<String> {
    client.stream(&prompt)
        .next()
        .await
        .ok_or(OptillmError::ClientError(
            "No response".to_string()
        ))?;

    Ok(String::new())
}
```

## Error Context

Add context when converting errors:

```rust
pub async fn call_llm(client: &dyn ModelClient) -> Result<String> {
    client.stream(&prompt)
        .next()
        .await
        .ok_or_else(|| OptillmError::ClientError(
            format!(
                "LLM call failed for query: '{}' with model: '{}'",
                query, model_id
            )
        ))?;

    Ok(String::new())
}
```

## Best Practices

### 1. Provide Context

```rust
// Good - explains what failed
Err(OptillmError::ClientError(
    format!("Failed to connect to {}: {}", base_url, error)
))

// Bad - no context
Err(OptillmError::ClientError("Error".to_string()))
```

### 2. Fail Fast

```rust
// Good - exit early on error
pub async fn process(config: &Config) -> Result<()> {
    validate_config(config)?;
    let client = create_client(config)?;
    let solution = solve(query, &client).await?;
    Ok(())
}
```

### 3. Avoid Unwrap/Panic

```rust
// Bad - panics on error
let solution = my_optimizer.optimize(q, c).await.unwrap();

// Good - returns error
let solution = my_optimizer.optimize(q, c).await?;
```

### 4. Match on Error Types

```rust
match my_function().await {
    Ok(result) => println!("Success: {}", result),
    Err(OptillmError::InvalidConfiguration(msg)) => {
        eprintln!("Bad config: {}", msg);
    }
    Err(OptillmError::ClientError(msg)) => {
        eprintln!("Client error: {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

### 5. Custom Error Types for Libraries

If creating a library on top of optillm-rs:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("OptimLLM error: {0}")]
    OptimLLM(#[from] OptillmError),

    #[error("Custom error: {0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, MyError>;
```

## Testing Error Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_config_error() {
        let config = invalid_config();
        let result = validate_config(&config);

        match result {
            Err(OptillmError::InvalidConfiguration(msg)) => {
                assert!(msg.contains("Temperature"));
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_error_message() {
        let error = OptillmError::InvalidConfiguration(
            "test error".to_string()
        );
        assert_eq!(
            error.to_string(),
            "Invalid configuration: test error"
        );
    }
}
```

## Error Display

OptillmError implements `Display` via `thiserror`:

```rust
let error = OptillmError::InvalidConfiguration("foo".to_string());
println!("{}", error);  // "Invalid configuration: foo"
```

## Common Error Scenarios

| Scenario | Error Type | Cause |
|----------|-----------|-------|
| Bad config value | `InvalidConfiguration` | User provides invalid setting |
| Network failure | `ClientError` | Cannot reach LLM provider |
| Invalid response | `ParsingError` | Provider returns unexpected format |
| Takes too long | `Timeout` | Operation exceeds time limit |

See [OptimLLM API](../api/core.md) for complete error documentation.
