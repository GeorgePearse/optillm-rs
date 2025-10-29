//! Comprehensive benchmark for all optimization strategies
//!
//! Tests all strategies across multiple models and tasks, generating
//! detailed performance metrics and recommendations.
//!
//! Run: cargo run --example comprehensive_benchmark --release
//!
//! Prerequisites:
//! - Ollama running: systemctl status ollama
//! - Models pulled: ollama pull tinyllama, ollama pull neural-chat

use optillm_mars::{
    providers::ollama::{OllamaClient, OllamaConfig},
    ReReadAggregator, ReReadConfig,
    DiverseSamplingAggregator, DiverseSamplingConfig,
    BestOfNAggregator, BestOfNConfig,
};
use serde::{Serialize, Deserialize};
use std::time::Instant;
use futures::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestTask {
    id: String,
    prompt: String,
    system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelConfig {
    name: String,
    size: String,
    category: String,
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkResult {
    strategy: String,
    model: String,
    task_id: String,
    tokens: u32,
    latency_ms: f32,
    throughput: f32,
    cost_multiplier: f32,
    success: bool,
    error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StrategyStats {
    strategy: String,
    avg_tokens: f32,
    avg_latency_ms: f32,
    avg_throughput: f32,
    cost_multiplier: f32,
    success_rate: f32,
    run_count: usize,
}

const TEST_TASKS: &[(&str, &str, &str)] = &[
    (
        "prime_check",
        "Write a Rust function that checks if a number is prime. Keep it under 15 lines.",
        "You are a Rust programmer. Write clean, correct, idiomatic code.",
    ),
    (
        "fibonacci",
        "Write a Rust function that returns the nth Fibonacci number. Keep it under 10 lines.",
        "You are a Rust programmer. Write efficient code.",
    ),
];

const MODELS: &[(&str, &str, &str)] = &[
    ("tinyllama", "1.1B", "tiny"),
];

const STRATEGIES: &[&str] = &[
    "baseline",
    "reread",
    "diverse_sampling",
    "best_of_n",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║      OptILLM-RS Comprehensive Strategy Benchmark               ║");
    println!("║    Testing all strategies across multiple models and tasks     ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Create clients for each model
    let mut clients: std::collections::HashMap<String, Box<dyn optillm_core::ModelClient>> =
        std::collections::HashMap::new();

    for (model_name, _, _) in MODELS {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            model_name.to_string(),
        )
        .with_temperature(0.3)
        .with_num_predict(512);

        match config.validate() {
            Ok(_) => {
                match OllamaClient::new(config) {
                    Ok(client) => {
                        println!("✓ Connected to {}", model_name);
                        clients.insert(
                            model_name.to_string(),
                            Box::new(client) as Box<dyn optillm_core::ModelClient>,
                        );
                    }
                    Err(e) => {
                        println!("✗ Failed to connect to {}: {}", model_name, e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Invalid config for {}: {}", model_name, e);
            }
        }
    }

    println!();

    // Store results
    let mut results = Vec::new();

    // Run benchmarks
    let total_tests = STRATEGIES.len() * MODELS.len() * TEST_TASKS.len();
    let mut completed = 0;

    for strategy in STRATEGIES {
        println!("{}", "=".repeat(70));
        println!("Strategy: {}", strategy.to_uppercase());
        println!("{}", "=".repeat(70));

        for (model_name, model_size, _) in MODELS {
            println!("\nModel: {} ({})", model_name, model_size);

            let client = match clients.get(*model_name) {
                Some(c) => c,
                None => {
                    println!("  [SKIP] Model not available");
                    continue;
                }
            };

            for (task_id, prompt, system_prompt) in TEST_TASKS {
                completed += 1;
                print!("  [{}/{}] Task {}... ", completed, total_tests, task_id);
                std::io::Write::flush(&mut std::io::stdout()).ok();

                let result = run_strategy_test(
                    *strategy,
                    *model_name,
                    *task_id,
                    prompt,
                    system_prompt,
                    client.as_ref(),
                )
                .await;

                if result.success {
                    println!(
                        "✓ {} tokens, {:.0}ms, {:.1} tok/s",
                        result.tokens, result.latency_ms, result.throughput
                    );
                } else {
                    println!("✗ {}", result.error);
                }

                results.push(result);
            }
        }

        println!();
    }

    // Generate report
    println!("{}", "=".repeat(70));
    println!("BENCHMARK RESULTS");
    println!("{}", "=".repeat(70));

    generate_report(&results).await;

    Ok(())
}

async fn run_strategy_test(
    strategy: &str,
    model: &str,
    task_id: &str,
    prompt: &str,
    system_prompt: &str,
    client: &dyn optillm_core::ModelClient,
) -> BenchmarkResult {
    let start = Instant::now();
    let cost_multiplier = get_cost_multiplier(strategy);

    let result = match strategy {
        "baseline" => run_baseline(prompt, system_prompt, client).await,
        "reread" => run_reread(prompt, system_prompt, client).await,
        "diverse_sampling" => run_diverse_sampling(prompt, system_prompt, client).await,
        "best_of_n" => run_best_of_n(prompt, system_prompt, client).await,
        _ => {
            return BenchmarkResult {
                strategy: strategy.to_string(),
                model: model.to_string(),
                task_id: task_id.to_string(),
                tokens: 0,
                latency_ms: 0.0,
                throughput: 0.0,
                cost_multiplier,
                success: false,
                error: format!("Unknown strategy: {}", strategy),
            };
        }
    };

    let elapsed = start.elapsed().as_secs_f32() * 1000.0;
    let throughput = if elapsed > 0.0 {
        (result.0 as f32 / elapsed) * 1000.0
    } else {
        0.0
    };

    BenchmarkResult {
        strategy: strategy.to_string(),
        model: model.to_string(),
        task_id: task_id.to_string(),
        tokens: result.0,
        latency_ms: elapsed,
        throughput,
        cost_multiplier,
        success: true,
        error: String::new(),
    }
}

async fn run_baseline(
    prompt: &str,
    system_prompt: &str,
    client: &dyn optillm_core::ModelClient,
) -> (u32, String) {
    let mut prompt_msg = optillm_core::Prompt::new();
    prompt_msg.input = vec![
        optillm_core::ResponseItem::Message {
            id: None,
            role: "system".to_string(),
            content: vec![optillm_core::ContentItem::InputText {
                text: system_prompt.to_string(),
            }],
        },
        optillm_core::ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![optillm_core::ContentItem::InputText {
                text: prompt.to_string(),
            }],
        },
    ];

    let mut stream = client.stream(&prompt_msg);
    let mut output = String::new();
    let mut tokens: u32 = 0;

    while let Some(event) = stream.next().await {
        if let Ok(optillm_core::ResponseEvent::OutputTextDelta { delta }) = event {
            output.push_str(&delta);
            tokens += 1;
        } else if let Ok(optillm_core::ResponseEvent::Completed { token_usage }) = event {
            if let Some(usage) = token_usage {
                tokens = usage.total_tokens() as u32;
            }
        }
    }

    (tokens, output)
}

async fn run_reread(
    prompt: &str,
    system_prompt: &str,
    client: &dyn optillm_core::ModelClient,
) -> (u32, String) {
    match ReReadAggregator::run_reread(
        prompt,
        system_prompt,
        ReReadConfig::new(),
        client,
    )
    .await
    {
        Ok(result) => (result.metadata.total_tokens as u32, result.answer),
        Err(_) => (0, String::new()),
    }
}

async fn run_diverse_sampling(
    prompt: &str,
    system_prompt: &str,
    client: &dyn optillm_core::ModelClient,
) -> (u32, String) {
    let config = DiverseSamplingConfig::new()
        .with_num_samples(3)
        .with_min_temperature(0.1)
        .with_max_temperature(0.9);

    match DiverseSamplingAggregator::run_diverse_sampling(
        prompt,
        system_prompt,
        config,
        client,
    )
    .await
    {
        Ok(result) => (result.metadata.total_tokens as u32, result.best_answer),
        Err(_) => (0, String::new()),
    }
}

async fn run_best_of_n(
    prompt: &str,
    system_prompt: &str,
    client: &dyn optillm_core::ModelClient,
) -> (u32, String) {
    match BestOfNAggregator::run_best_of_n(
        prompt,
        system_prompt,
        BestOfNConfig::new(3),
        client,
    )
    .await
    {
        Ok((solution, metadata)) => (metadata.total_tokens as u32, solution.answer),
        Err(_) => (0, String::new()),
    }
}

fn get_cost_multiplier(strategy: &str) -> f32 {
    match strategy {
        "baseline" => 1.0,
        "reread" => 1.5,
        "diverse_sampling" => 3.0,
        "best_of_n" => 3.0,
        _ => 1.0,
    }
}

async fn generate_report(results: &[BenchmarkResult]) {
    // Calculate strategy statistics
    let mut strategy_stats: std::collections::HashMap<String, Vec<&BenchmarkResult>> =
        std::collections::HashMap::new();

    for result in results {
        strategy_stats
            .entry(result.strategy.clone())
            .or_insert_with(Vec::new)
            .push(result);
    }

    println!("\n{}", "=".repeat(70));
    println!("STRATEGY COMPARISON TABLE");
    println!("{}", "=".repeat(70));
    println!(
        "{:<20} {:<12} {:<12} {:<12} {:<12} {:<10}",
        "Strategy", "Avg Tokens", "Avg Latency", "Throughput", "Cost", "Success%"
    );
    println!("{}", "-".repeat(70));

    for strategy in STRATEGIES {
        if let Some(results) = strategy_stats.get(*strategy) {
            let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
            if !successful.is_empty() {
                let avg_tokens = successful.iter().map(|r| r.tokens as f32).sum::<f32>()
                    / successful.len() as f32;
                let avg_latency = successful.iter().map(|r| r.latency_ms).sum::<f32>()
                    / successful.len() as f32;
                let avg_throughput = successful.iter().map(|r| r.throughput).sum::<f32>()
                    / successful.len() as f32;
                let cost = results[0].cost_multiplier;
                let success_rate = (successful.len() as f32 / results.len() as f32) * 100.0;

                println!(
                    "{:<20} {:<12.0} {:<12.1}ms {:<12.1} {:<12.1}x {:<10.1}%",
                    strategy, avg_tokens, avg_latency, avg_throughput, cost, success_rate
                );
            }
        }
    }

    // Model comparison
    println!("\n{}", "=".repeat(70));
    println!("RESULTS BY MODEL");
    println!("{}", "=".repeat(70));

    let mut by_model: std::collections::HashMap<String, Vec<&BenchmarkResult>> =
        std::collections::HashMap::new();

    for result in results {
        by_model
            .entry(result.model.clone())
            .or_insert_with(Vec::new)
            .push(result);
    }

    for (model, model_results) in by_model {
        println!("\n{}", model);
        println!("{}", "-".repeat(70));
        println!(
            "{:<20} {:<15} {:<15} {:<15}",
            "Strategy", "Latency (ms)", "Throughput", "Status"
        );

        for result in model_results.iter().filter(|r| r.task_id == "prime_check") {
            let status = if result.success { "✓" } else { "✗" };
            println!(
                "{:<20} {:<15.1} {:<15.1} {:<15}",
                result.strategy, result.latency_ms, result.throughput, status
            );
        }
    }

    // Recommendations
    println!("\n{}", "=".repeat(70));
    println!("RECOMMENDATIONS");
    println!("{}", "=".repeat(70));

    let successful: Vec<_> = results.iter().filter(|r| r.success).collect();
    if !successful.is_empty() {
        let fastest = successful.iter().min_by(|a, b| {
            a.latency_ms.partial_cmp(&b.latency_ms).unwrap()
        });
        if let Some(fast) = fastest {
            println!("✓ Fastest strategy: {} ({:.0}ms)", fast.strategy, fast.latency_ms);
        }

        let best_efficiency = successful.iter().min_by(|a, b| {
            (a.latency_ms * a.cost_multiplier)
                .partial_cmp(&(b.latency_ms * b.cost_multiplier))
                .unwrap()
        });
        if let Some(eff) = best_efficiency {
            println!(
                "✓ Most efficient: {} ({:.1}x cost, {:.0}ms adjusted latency)",
                eff.strategy,
                eff.cost_multiplier,
                eff.latency_ms * eff.cost_multiplier
            );
        }
    }

    println!("\n✓ Benchmark complete!");
}
