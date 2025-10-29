//! Test optimization strategies with TinyLlama 1.1B
//!
//! Demonstrates how optimization strategies improve output quality
//! from a small model.
//!
//! Run: cargo run --example tinyllama_strategies
//!
//! Prerequisites:
//! - Ollama running: systemctl status ollama
//! - TinyLlama pulled: ollama pull tinyllama

use optillm_mars::{
    providers::ollama::{OllamaClient, OllamaConfig},
    ReReadAggregator, ReReadConfig,
    DiverseSamplingAggregator, DiverseSamplingConfig,
    BestOfNAggregator, BestOfNConfig,
};
use std::time::Instant;

const CODING_TASK: &str = "Write a Rust function that checks if a number is prime. Keep it under 15 lines.";

const SYSTEM_PROMPT: &str = "You are a Rust programmer. Write clean, correct, idiomatic code.";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        Testing Optimization Strategies with TinyLlama 1.1B     ║");
    println!("║         Comparing Strategy Effectiveness on Small Model        ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    // Create TinyLlama client
    let config = OllamaConfig::new(
        "http://localhost:11434".to_string(),
        "tinyllama".to_string(),
    )
    .with_temperature(0.3)
    .with_num_predict(512);

    config.validate()?;
    let client = OllamaClient::new(config)?;

    println!("✓ Connected to TinyLlama 1.1B");
    println!("  Model: tinyllama");
    println!("  Temperature: 0.3 (deterministic)");
    println!("  Max tokens: 512\n");

    println!("Task: {}\n", CODING_TASK);

    // Test 1: Baseline (No optimization)
    println!("{}", "=".repeat(70));
    println!("TEST 1: Baseline (Single Pass, No Optimization)");
    println!("{}", "=".repeat(70));
    test_baseline(&client).await?;

    // Test 2: ReRead Strategy
    println!("\n{}", "=".repeat(70));
    println!("TEST 2: ReRead Strategy (Simple Re-reading)");
    println!("{}", "=".repeat(70));
    test_reread(&client).await?;

    // Test 3: Diverse Sampling
    println!("\n{}", "=".repeat(70));
    println!("TEST 3: Diverse Sampling (Temperature Variation)");
    println!("{}", "=".repeat(70));
    test_diverse_sampling(&client).await?;

    // Test 4: Best-of-N
    println!("\n{}", "=".repeat(70));
    println!("TEST 4: Best-of-N (Multiple Attempts, Select Best)");
    println!("{}", "=".repeat(70));
    test_best_of_n(&client).await?;

    // Summary
    println!("\n{}", "=".repeat(70));
    println!("SUMMARY & ANALYSIS");
    println!("{}", "=".repeat(70));

    println!("\n📊 Strategy Comparison:");
    println!("  Baseline:         1x cost, baseline quality");
    println!("  ReRead:           1x cost, slightly better (verification)");
    println!("  Diverse Sampling: 3x cost, explores temperature space");
    println!("  Best-of-N:        3x cost, selects best from multiple");

    println!("\n💡 Recommendations for TinyLlama:");
    println!("  ✓ Use ReRead for slight quality improvement (no extra cost)");
    println!("  ✓ Use Diverse Sampling to explore different styles");
    println!("  ✓ Use Best-of-N when you want to select highest quality");
    println!("  ✗ Avoid heavy strategies (too expensive for 1.1B model)");

    println!("\n🚀 Next Steps:");
    println!("  1. Try larger models for better baseline quality");
    println!("  2. Test strategies on your specific use cases");
    println!("  3. Measure quality improvements vs. cost trade-off");
    println!("  4. Optimize strategy parameters for your needs\n");

    Ok(())
}

async fn test_baseline(client: &dyn optillm_core::ModelClient) -> Result<(), Box<dyn std::error::Error>> {
    use futures::StreamExt;

    println!("\nRunning single inference...");

    let mut prompt = optillm_core::Prompt::new();
    prompt.input = vec![
        optillm_core::ResponseItem::Message {
            id: None,
            role: "system".to_string(),
            content: vec![optillm_core::ContentItem::InputText {
                text: SYSTEM_PROMPT.to_string(),
            }],
        },
        optillm_core::ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![optillm_core::ContentItem::InputText {
                text: CODING_TASK.to_string(),
            }],
        },
    ];

    let start = Instant::now();
    let mut stream = client.stream(&prompt);
    let mut output = String::new();
    let mut tokens = 0;

    while let Some(event) = stream.next().await {
        match event? {
            optillm_core::ResponseEvent::OutputTextDelta { delta } => {
                output.push_str(&delta);
                tokens += 1;
            }
            optillm_core::ResponseEvent::Completed { token_usage } => {
                if let Some(usage) = token_usage {
                    tokens = usage.total_tokens();
                }
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f32();

    println!("\nGenerated Code:");
    println!("{}", "-".repeat(70));
    println!("{}", output);
    println!("{}", "-".repeat(70));

    println!("\nMetrics:");
    println!("  Tokens: {}", tokens);
    println!("  Time: {:.1}s", elapsed);
    println!("  Speed: {:.0} tokens/sec", tokens as f32 / elapsed);
    println!("  Cost: 1x (baseline)");

    Ok(())
}

async fn test_reread(client: &dyn optillm_core::ModelClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nRunning ReRead strategy...");
    println!("(Re-reads and verifies the generated code)");

    let start = Instant::now();
    let result = ReReadAggregator::run_reread(
        CODING_TASK,
        SYSTEM_PROMPT,
        ReReadConfig::new(),
        client,
    ).await?;
    let elapsed = start.elapsed().as_secs_f32();

    println!("\nGenerated Code:");
    println!("{}", "-".repeat(70));
    println!("{}", result.answer);
    println!("{}", "-".repeat(70));

    println!("\nMetrics:");
    println!("  Tokens: {}", result.metadata.total_tokens);
    println!("  Time: {:.1}s", elapsed);
    println!("  Speed: {:.0} tokens/sec", result.metadata.total_tokens as f32 / elapsed);
    println!("  Cost: ~1x (verification pass)");
    println!("  Quality: Similar to baseline + verification");

    Ok(())
}

async fn test_diverse_sampling(client: &dyn optillm_core::ModelClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nRunning Diverse Sampling strategy...");
    println!("(Generates at 3 different temperatures: 0.1, 0.5, 0.9)");

    let config = DiverseSamplingConfig::new()
        .with_num_samples(3)
        .with_min_temperature(0.1)
        .with_max_temperature(0.9);

    let start = Instant::now();
    let result = DiverseSamplingAggregator::run_diverse_sampling(
        CODING_TASK,
        SYSTEM_PROMPT,
        config,
        client,
    ).await?;
    let elapsed = start.elapsed().as_secs_f32();

    println!("\nSelected Best Answer (from 3 temperatures):");
    println!("{}", "-".repeat(70));
    println!("{}", result.best_answer);
    println!("{}", "-".repeat(70));

    println!("\nAll Temperatures Generated:");
    for (i, sample) in result.metadata.samples.iter().enumerate() {
        println!("\n  Sample {}: Temperature {:.1}", i + 1, sample.temperature);
        println!("    {} tokens, first line: {}",
            sample.tokens,
            sample.answer.lines().next().unwrap_or("(empty)")
        );
    }

    println!("\nMetrics:");
    println!("  Total tokens: {}", result.metadata.total_tokens);
    println!("  Unique answers: {}", result.metadata.unique_answers);
    println!("  Time: {:.1}s", elapsed);
    println!("  Speed: {:.0} tokens/sec", result.metadata.total_tokens as f32 / elapsed);
    println!("  Cost: 3x (3 temperature levels)");
    println!("  Quality: Explores response space, selects best style");

    Ok(())
}

async fn test_best_of_n(client: &dyn optillm_core::ModelClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nRunning Best-of-N strategy...");
    println!("(Generates 3 attempts, selects best)");

    let config = BestOfNConfig::new(3);

    let start = Instant::now();
    let (solution, metadata) = BestOfNAggregator::run_best_of_n(
        CODING_TASK,
        SYSTEM_PROMPT,
        config,
        client,
    ).await?;
    let elapsed = start.elapsed().as_secs_f32();

    println!("\nSelected Best Answer (from 3 attempts):");
    println!("{}", "-".repeat(70));
    println!("{}", solution.answer);
    println!("{}", "-".repeat(70));

    println!("\nSelection Metrics:");
    println!("  Selection method: {}", metadata.selection_method);
    println!("  Best answer length: {} chars", solution.answer.len());

    println!("\nPerformance Metrics:");
    println!("  Total tokens: {}", metadata.total_tokens);
    println!("  Time: {:.1}s", elapsed);
    println!("  Speed: {:.0} tokens/sec", metadata.total_tokens as f32 / elapsed);
    println!("  Cost: 3x (3 attempts)");
    println!("  Quality: Best of 3 samples");

    Ok(())
}
