//! Test ultra-tiny models on resource-constrained systems
//!
//! Tests the smallest practical models:
//! - TinyLlama 1.1B (1GB VRAM)
//! - DistilGPT-2 82M (200MB VRAM)
//! - Qwen 0.5B (800MB VRAM)
//!
//! These run on:
//! - Old laptops with 2GB RAM
//! - Raspberry Pi 4
//! - Mobile devices
//! - IoT systems
//! - CPU-only systems
//!
//! Setup:
//! 1. Install Ollama: https://ollama.ai
//! 2. Pull models:
//!    ollama pull tinyllama
//!    ollama pull gpt2
//! 3. Run: cargo run --example test_ultra_tiny

use std::time::Instant;
use optillm_mars::providers::ollama::{OllamaClient, OllamaConfig};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};
use futures::StreamExt;

const SIMPLE_TASK: &str = "Write a simple function in Rust. Keep it under 10 lines.";

const SYSTEM_PROMPT: &str = "You are helpful. Write simple, clear code.";

struct UltraTinyModel {
    name: &'static str,
    model_id: &'static str,
    min_vram_mb: usize,
    expected_tokens_per_sec: f32,
}

const ULTRA_TINY_MODELS: &[UltraTinyModel] = &[
    UltraTinyModel {
        name: "TinyLlama 1.1B",
        model_id: "tinyllama",
        min_vram_mb: 1024,
        expected_tokens_per_sec: 60.0,
    },
    UltraTinyModel {
        name: "DistilGPT-2 82M",
        model_id: "gpt2",
        min_vram_mb: 200,
        expected_tokens_per_sec: 100.0,
    },
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║         Ultra-Tiny Models - Resource-Constrained Test         ║");
    println!("║         Perfect for Raspberry Pi, IoT, Old Laptops            ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("SYSTEM REQUIREMENTS:");
    println!("  TinyLlama 1.1B: ~1-2GB VRAM minimum");
    println!("  DistilGPT-2 82M: ~200-500MB VRAM minimum");
    println!("  Both work great on CPU!\n");

    println!("SETUP:");
    println!("  1. ollama serve");
    println!("  2. ollama pull tinyllama");
    println!("  3. ollama pull gpt2");
    println!("  4. cargo run --example test_ultra_tiny\n");

    println!("{}", "=".repeat(70));
    println!("Testing Ultra-Tiny Models");
    println!("{}", "=".repeat(70));

    for model in ULTRA_TINY_MODELS {
        println!("\n{}", "=".repeat(70));
        println!("Model: {}", model.name);
        println!("  Model ID: {}", model.model_id);
        println!("  Min VRAM: ~{}MB", model.min_vram_mb);
        println!("  Expected Speed: ~{:.0} tokens/sec", model.expected_tokens_per_sec);
        println!("{}", "=".repeat(70));

        match test_model(model).await {
            Ok(result) => {
                println!("\n✓ SUCCESS");
                println!("  Generated: {} tokens", result.tokens);
                println!("  Time: {:.2}s", result.elapsed);
                println!("  Speed: {:.1} tokens/sec", result.tokens as f32 / result.elapsed);
                println!("\nGenerated code:");
                println!("{}", "-".repeat(70));
                println!("{}", result.output);
                println!("{}", "-".repeat(70));
            }
            Err(e) => {
                println!("\n✗ FAILED: {}", e);
                println!("  Ensure Ollama is running: ollama serve");
                println!("  Ensure model is pulled: ollama pull {}", model.model_id);
            }
        }
    }

    println!("\n{}", "=".repeat(70));
    println!("ULTRA-TINY MODEL RECOMMENDATIONS");
    println!("{}", "=".repeat(70));

    println!("\n🏆 TinyLlama 1.1B");
    println!("  ✓ Smallest usable full-featured model");
    println!("  ✓ 1GB VRAM requirement");
    println!("  ✓ Good instruction following");
    println!("  ✓ Works on Raspberry Pi 4");
    println!("  ✓ Best balance");

    println!("\n⚡ DistilGPT-2 82M");
    println!("  ✓ Literally the smallest working model");
    println!("  ✓ Only 200MB VRAM");
    println!("  ✓ Extremely fast (100+ tok/sec on CPU)");
    println!("  ✓ Works on old laptops");
    println!("  ✓ Very limited capabilities");

    println!("\n📊 USE CASES:");
    println!("\n  Raspberry Pi 4 (4GB RAM):");
    println!("    → Use TinyLlama");
    println!("    → Run with: OLLAMA_NUM_PARALLEL=1 ollama serve");

    println!("\n  Old Laptop (2GB RAM):");
    println!("    → Use DistilGPT-2 or TinyLlama");
    println!("    → Close other applications");

    println!("\n  IoT Device (256MB RAM):");
    println!("    → Use DistilGPT-2");
    println!("    → May need llama.cpp instead of Ollama");

    println!("\n💡 OPTIMIZATION STRATEGIES FOR TINY MODELS:");

    println!("\n  Recommended:");
    println!("    ✓ Single generation (fastest)");
    println!("    ✓ ReRead (simple verification)");
    println!("    ✓ Diverse Sampling (with 2-3 samples)");

    println!("\n  Not Recommended (too expensive):");
    println!("    ✗ Self-Consistency (5+ paths)");
    println!("    ✗ PVG (too complex)");
    println!("    ✗ RSA (too many passes)");

    println!("\n🚀 QUICK START:");
    println!("  1. Use TinyLlama for 1GB+ VRAM");
    println!("  2. Use DistilGPT-2 for <500MB VRAM");
    println!("  3. Use environment variable to switch:");
    println!("     LLM_MODEL=tinyllama cargo build");
    println!("     LLM_MODEL=gpt2 cargo build");

    println!("\n💾 DISK SPACE NEEDED:");
    println!("  DistilGPT-2: ~160MB");
    println!("  TinyLlama: ~1.2GB");
    println!("  Both easily fit on SD cards!\n");

    Ok(())
}

struct TestResult {
    output: String,
    tokens: usize,
    elapsed: f32,
}

async fn test_model(model: &UltraTinyModel) -> Result<TestResult, Box<dyn std::error::Error>> {
    let config = OllamaConfig::new(
        "http://localhost:11434".to_string(),
        model.model_id.to_string(),
    )
    .with_temperature(0.5)
    .with_num_predict(256);  // Keep small to test on constrained systems

    config.validate()?;

    let client = OllamaClient::new(config)?;

    let mut prompt = Prompt::new();
    prompt.input = vec![
        ResponseItem::Message {
            id: None,
            role: "system".to_string(),
            content: vec![ContentItem::InputText {
                text: SYSTEM_PROMPT.to_string(),
            }],
        },
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: SIMPLE_TASK.to_string(),
            }],
        },
    ];

    let start = Instant::now();
    let mut stream = client.stream(&prompt);
    let mut output = String::new();
    let mut tokens = 0;

    while let Some(event) = stream.next().await {
        match event? {
            ResponseEvent::OutputTextDelta { delta } => {
                output.push_str(&delta);
                tokens += 1;
            }
            ResponseEvent::Completed { token_usage } => {
                if let Some(usage) = token_usage {
                    tokens = usage.total_tokens();
                }
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f32();

    if tokens == 0 {
        return Err("No tokens generated".into());
    }

    Ok(TestResult {
        output,
        tokens,
        elapsed,
    })
}
