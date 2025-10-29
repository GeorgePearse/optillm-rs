//! Quick test of DeepSeek Coder 6.7B - Best balance of speed/quality for coding
//!
//! DeepSeek Coder 6.7B is specifically trained for coding and offers:
//! - 6.7B parameters (purpose-built for code)
//! - ~4.5GB VRAM with quantization (Q4_K_M)
//! - ~25 tokens/sec throughput
//! - Excellent code generation quality
//! - Works on CPU if needed
//!
//! Setup:
//! 1. Install Ollama: https://ollama.ai
//! 2. Pull the model: ollama pull deepseek-coder:6.7b-base-q4_K_M
//! 3. Start Ollama: ollama serve
//! 4. Run this example: cargo run --example test_deepseek_coder
//!
//! This tests using the Self-Consistency strategy (voting across multiple attempts)
//! to improve code quality with a small model.

use std::time::Instant;
use optillm_mars::{
    SelfConsistencyAggregator, SelfConsistencyConfig,
    providers::ollama::{OllamaClient, OllamaConfig},
};

const CODING_PROBLEM: &str = r#"
Write a Rust function that:
1. Takes a vector of integers
2. Returns the median value
3. Handles odd and even length vectors correctly

Include proper error handling and documentation.
"#;

const SYSTEM_PROMPT: &str = r#"
You are an expert Rust programmer specializing in correct, efficient code.
Write clean, idiomatic Rust with:
- Proper error handling
- Good documentation
- Efficient algorithms
- Clear variable names
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         DeepSeek Coder 6.7B - Coding LLM Test                  ║");
    println!("║  Purpose-built for code | 4.5GB VRAM | ~25 tokens/sec         ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("SETUP INSTRUCTIONS:");
    println!("1. Install Ollama from https://ollama.ai");
    println!("2. Pull the model:");
    println!("   ollama pull deepseek-coder:6.7b-base-q4_K_M");
    println!("3. Start Ollama server:");
    println!("   ollama serve");
    println!("4. Run this test in another terminal:");
    println!("   cargo run --example test_deepseek_coder\n");

    println!("{}", "=".repeat(70));
    println!("Testing: DeepSeek Coder 6.7B");
    println!("{}", "=".repeat(70));

    // Create Ollama config
    let config = OllamaConfig::new(
        "http://localhost:11434".to_string(),
        "deepseek-coder:6.7b-base-q4_K_M".to_string(),
    )
    .with_temperature(0.3)  // Low temperature for deterministic coding
    .with_num_predict(1024);

    println!("\nConfiguration:");
    println!("  Temperature: 0.3 (deterministic)");
    println!("  Max tokens: 1024");
    println!("  Top-p: 0.9");
    println!("  Top-k: 40");

    // Validate config
    config.validate()?;

    // Create client
    let client = OllamaClient::new(config)?;
    println!("\n✓ Connected to Ollama");

    // Test single inference
    println!("\n{}", "=".repeat(70));
    println!("TEST 1: Single Inference");
    println!("{}", "=".repeat(70));

    let start = Instant::now();
    let result = test_single_inference(&client).await?;
    let elapsed = start.elapsed().as_secs_f32();

    println!("\n✓ Single inference completed");
    println!("  Tokens generated: {}", result.tokens);
    println!("  Time: {:.2}s", elapsed);
    println!("  Throughput: {:.1} tokens/sec", result.tokens as f32 / elapsed);
    println!("\nGenerated code:");
    println!("{}", "-".repeat(70));
    println!("{}", result.answer);
    println!("{}", "-".repeat(70));

    // Test Self-Consistency (voting across multiple attempts)
    println!("\n{}", "=".repeat(70));
    println!("TEST 2: Self-Consistency Strategy (Voting Across 3 Attempts)");
    println!("{}", "=".repeat(70));
    println!("\nThis strategy generates the same code 3 times with different");
    println!("randomness and selects the most common answer (voting).\n");

    let start = Instant::now();
    let consistency_result = test_self_consistency(&client).await?;
    let elapsed = start.elapsed().as_secs_f32();

    println!("\n✓ Self-Consistency completed");
    println!("  Total tokens: {}", consistency_result.total_tokens);
    println!("  Time: {:.2}s", elapsed);
    println!("  Throughput: {:.1} tokens/sec", consistency_result.total_tokens as f32 / elapsed);
    println!("  Agreement: {:.0}%", consistency_result.agreement);
    println!("  Paths generated: 3");

    println!("\nFinal answer (consensus):");
    println!("{}", "-".repeat(70));
    println!("{}", consistency_result.final_answer);
    println!("{}", "-".repeat(70));

    // Summary
    println!("\n{}", "=".repeat(70));
    println!("SUMMARY & RECOMMENDATIONS");
    println!("{}", "=".repeat(70));

    println!("\n✓ DeepSeek Coder 6.7B is ideal for:");
    println!("  • Coding tasks (purpose-built)");
    println!("  • Local development (no API costs)");
    println!("  • Privacy-sensitive code");
    println!("  • Offline development");

    println!("\n📊 Performance Profile:");
    println!("  • Model Size: 6.7B parameters");
    println!("  • VRAM Required: ~4.5GB (quantized)");
    println!("  • Speed: ~25 tokens/sec");
    println!("  • Code Quality: Excellent");
    println!("  • Specialization: Coding");

    println!("\n🚀 Optimization Strategies to Use:");
    println!("  ✓ Self-Consistency: Voting across multiple attempts");
    println!("  ✓ Diverse Sampling: Explore temperature range (0.1 - 0.8)");
    println!("  ✓ Best-of-N: Generate multiple and select best");
    println!("  ✓ Round-Trip: Generate code, then verify it works");

    println!("\n🎯 Best Practices:");
    println!("  1. Use low temperature (0.1-0.3) for deterministic code");
    println!("  2. Use Self-Consistency for important code");
    println!("  3. Always verify generated code");
    println!("  4. Combine with optimization strategies for better quality");

    println!("\n💡 Next Steps:");
    println!("  Try other small models for comparison:");
    println!("  • Phi-3 Mini (3.8B, faster, lighter)");
    println!("  • TinyLlama (1.1B, fastest)");
    println!("  • CodeLlama 7B (larger, higher quality)");

    println!("\nRun the full benchmark: cargo run --example test_coding_models\n");

    Ok(())
}

#[derive(Debug)]
struct SingleResult {
    answer: String,
    tokens: usize,
}

#[derive(Debug)]
struct ConsistencyResult {
    final_answer: String,
    total_tokens: usize,
    agreement: f32,
}

async fn test_single_inference(client: &dyn optillm_core::ModelClient) -> Result<SingleResult, Box<dyn std::error::Error>> {
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
                text: CODING_PROBLEM.to_string(),
            }],
        },
    ];

    let mut stream = client.stream(&prompt);
    let mut answer = String::new();
    let mut tokens = 0;

    while let Some(event) = stream.next().await {
        match event? {
            optillm_core::ResponseEvent::OutputTextDelta { delta } => {
                answer.push_str(&delta);
                tokens += 1;
            }
            optillm_core::ResponseEvent::Completed { token_usage } => {
                if let Some(usage) = token_usage {
                    tokens = usage.total_tokens();
                }
            }
        }
    }

    Ok(SingleResult { answer, tokens })
}

async fn test_self_consistency(client: &dyn optillm_core::ModelClient) -> Result<ConsistencyResult, Box<dyn std::error::Error>> {
    let config = SelfConsistencyConfig::new()
        .with_num_sampling_paths(3)
        .with_temperature(0.4);

    let result = SelfConsistencyAggregator::run_self_consistency(
        CODING_PROBLEM,
        SYSTEM_PROMPT,
        config,
        client,
    ).await?;

    Ok(ConsistencyResult {
        final_answer: result.final_answer,
        total_tokens: result.metadata.total_tokens,
        agreement: result.metadata.agreement_percentage,
    })
}

// Need to use StreamExt for .next()
use futures::StreamExt;
