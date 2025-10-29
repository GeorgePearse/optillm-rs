# Examples

Practical examples of using optillm-rs for different optimization tasks.

## Simple Query Optimization

```rust
use optillm_mars::{MarsCoordinator, MarsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();
    let config = MarsConfig::default();
    let coordinator = MarsCoordinator::new(config);

    let query = "Solve this math problem: 17 * 23 = ?";
    let result = coordinator.optimize(query, &client).await?;

    println!("Answer: {}", result.answer);
    Ok(())
}
```

## Complex Reasoning with Verification

```rust
use optillm_mars::{MarsCoordinator, MarsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();

    let config = MarsConfig {
        num_agents: 7,
        temperatures: vec![0.3, 0.5, 0.7, 0.9, 1.0, 1.1, 1.2],
        max_iterations: 3,
        verification_threshold: 0.8,
        ..Default::default()
    };

    let coordinator = MarsCoordinator::new(config);

    let query = "Explain quantum entanglement and its implications for computing";
    let result = coordinator.optimize(query, &client).await?;

    println!("Query: {}", query);
    println!("Answer: {}", result.answer);
    println!("Verification passes: {}", result.all_solutions[0].verification_passes);

    Ok(())
}
```

## Using Mixture of Agents (MOA)

```rust
use optillm_mars::MoaAggregator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();

    let (solution, metadata) = MoaAggregator::run_moa(
        "Write a poem about autumn",
        "You are a creative poet.",
        3,  // Generate 3 diverse responses
        true,  // Enable fallback if one fails
        &client,
    ).await?;

    println!("Generated poem:\n{}", solution.answer);
    println!("\nMetadata:");
    println!("  Total tokens used: {}", metadata.total_tokens);
    println!("  Phase 1 tokens: {}", metadata.phase1_tokens);
    println!("  Phase 2 tokens: {}", metadata.phase2_tokens);
    println!("  Phase 3 tokens: {}", metadata.phase3_tokens);

    Ok(())
}
```

## Batch Processing Multiple Queries

```rust
use optillm_mars::{MarsCoordinator, MarsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();
    let config = MarsConfig::default();
    let coordinator = MarsCoordinator::new(config);

    let queries = vec![
        "What is photosynthesis?",
        "Explain machine learning",
        "Describe quantum mechanics",
    ];

    for (idx, query) in queries.iter().enumerate() {
        println!("\n--- Query {} ---", idx + 1);

        match coordinator.optimize(query, &client).await {
            Ok(result) => {
                println!("Q: {}", query);
                println!("A: {}", result.answer);
                println!("Iterations: {}", result.iterations);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

## Temperature Sweep Analysis

```rust
use optillm_mars::{MarsCoordinator, MarsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();

    let temperature_sets = vec![
        vec![0.1, 0.3, 0.5],  // Conservative
        vec![0.5, 0.7, 0.9],  // Moderate
        vec![0.9, 1.2, 1.5],  // Creative
    ];

    for (set_idx, temps) in temperature_sets.iter().enumerate() {
        let config = MarsConfig {
            num_agents: temps.len(),
            temperatures: temps.clone(),
            ..Default::default()
        };

        let coordinator = MarsCoordinator::new(config);
        let result = coordinator.optimize(
            "Generate a creative story idea",
            &client,
        ).await?;

        println!("Temperature set {}: {}", set_idx + 1, result.answer);
    }

    Ok(())
}
```

## Custom Provider Routing

```rust
use optillm_mars::provider_config::{ProviderSpec, ProviderRoutingConfig, RoutingStrategy};

fn setup_multi_provider_routing() -> ProviderRoutingConfig {
    let openai = ProviderSpec::new("openai", "gpt-4")
        .with_api_key("sk-xxx".to_string())
        .with_priority(1);

    let anthropic = ProviderSpec::new("anthropic", "claude-3-sonnet")
        .with_api_key("sk-ant-xxx".to_string())
        .with_priority(2);

    let groq = ProviderSpec::new("groq", "mixtral-8x7b")
        .with_api_key("gsk-xxx".to_string())
        .with_priority(3);

    ProviderRoutingConfig::multi(openai, vec![anthropic, groq])
        .with_strategy(RoutingStrategy::RoundRobin)
        .with_fallback(true)
        .with_max_retries(2)
}
```

## Strategy Network Learning

```rust
use optillm_mars::StrategyNetwork;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut strategy_network = StrategyNetwork::new();

    // Register strategies discovered by agents
    let strat1 = strategy_network.register_strategy(
        "agent-1".to_string(),
        "Break complex problems into smaller steps".to_string(),
        "Use chain-of-thought reasoning".to_string(),
    );

    let strat2 = strategy_network.register_strategy(
        "agent-2".to_string(),
        "Verify answers through multiple approaches".to_string(),
        "Cross-check with different methods".to_string(),
    );

    // Update success rates based on outcomes
    strategy_network.update_success_rate(&strat1, true)?;  // Success
    strategy_network.update_success_rate(&strat2, false)?; // Failed

    // Get top strategies
    let top_strategies = strategy_network.get_top_strategies(5);
    for (idx, strat) in top_strategies.iter().enumerate() {
        println!(
            "{}. {} (Success rate: {:.1}%)",
            idx + 1,
            strat.description,
            strat.success_rate * 100.0
        );
    }

    // Get diversity metrics
    let diversity = strategy_network.get_diversity_metrics();
    println!("\nDiversity Metrics:");
    println!("  Total strategies: {}", diversity.total_strategies);
    println!("  Unique agents: {}", diversity.unique_agents);
    println!("  Avg success rate: {:.1}%", diversity.avg_success_rate * 100.0);

    Ok(())
}
```

## Error Handling and Retry Logic

```rust
use optillm_mars::{MarsCoordinator, MarsConfig, MarsError};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();
    let config = MarsConfig::default();
    let coordinator = MarsCoordinator::new(config);

    let max_retries = 3;
    let mut retry_count = 0;

    loop {
        match coordinator.optimize("Your query", &client).await {
            Ok(result) => {
                println!("Success: {}", result.answer);
                break;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    eprintln!("Failed after {} retries: {}", max_retries, e);
                    break;
                }

                let wait_time = Duration::from_secs(u64::pow(2, retry_count as u32));
                println!("Attempt {} failed, retrying in {:?}...", retry_count, wait_time);
                sleep(wait_time).await;
            }
        }
    }

    Ok(())
}
```

## Logging and Monitoring

```rust
use optillm_mars::{MarsCoordinator, MarsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = your_model_client();
    let config = MarsConfig::default();
    let coordinator = MarsCoordinator::new(config);

    let start = std::time::Instant::now();
    let result = coordinator.optimize("Your query", &client).await?;
    let elapsed = start.elapsed();

    println!("Optimization completed in {:?}", elapsed);
    println!("Total tokens used: {}", result.total_tokens);
    println!("Iterations: {}", result.iterations);
    println!("Final answer: {}", result.answer);

    // Analyze solutions
    for (idx, solution) in result.all_solutions.iter().enumerate() {
        println!(
            "Solution {}: Score={:.2}, Verified={}, Temp={}",
            idx + 1,
            solution.verification_score,
            solution.is_verified,
            solution.temperature
        );
    }

    Ok(())
}
```

## Performance Benchmarking

```rust
use optillm_mars::{MarsCoordinator, MarsConfig};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = your_model_client();

    let configs = vec![
        ("Small", MarsConfig {
            num_agents: 3,
            ..Default::default()
        }),
        ("Medium", MarsConfig {
            num_agents: 5,
            ..Default::default()
        }),
        ("Large", MarsConfig {
            num_agents: 7,
            ..Default::default()
        }),
    ];

    for (name, config) in configs {
        let coordinator = MarsCoordinator::new(config);

        let start = Instant::now();
        let result = coordinator.optimize("Your query", &client).await?;
        let elapsed = start.elapsed();

        println!("{} configuration:", name);
        println!("  Time: {:?}", elapsed);
        println!("  Tokens: {}", result.total_tokens);
        println!("  Quality: {:.2}", result.all_solutions[0].verification_score);
    }

    Ok(())
}
```

## More Examples

See the `examples/` directory in the repository for additional working examples.
