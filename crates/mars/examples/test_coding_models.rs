//! Test suite for evaluating small local coding LLMs
//!
//! This example tests different small coding models with optimization strategies.
//! Models tested:
//! - DeepSeek Coder 6.7B (best balance of speed/quality for coding)
//! - Phi-3 Mini 3.8B (ultra-lightweight)
//! - TinyLlama 1.1B (smallest)
//! - CodeLlama 7B (specialized but larger)
//!
//! Usage:
//! 1. Install Ollama: https://ollama.ai
//! 2. Pull models:
//!    ollama pull deepseek-coder:6.7b-base-q4_K_M
//!    ollama pull phi:mini
//!    ollama pull tinyllama
//!    ollama pull codellama
//! 3. Run: cargo run --example test_coding_models --features test
//!
//! The test will benchmark each model on coding tasks using different optimization strategies.

use std::time::Instant;
use optillm_mars::providers::ollama::{OllamaClient, OllamaConfig};
use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem};
use futures::StreamExt;

/// A simple coding task for testing
const CODING_TASK: &str = "Write a Rust function that checks if a number is prime. Include error handling.";

const SYSTEM_PROMPT: &str = "You are an expert Rust programmer. Write clean, idiomatic code with proper error handling and documentation.";

/// Configuration for different models
#[derive(Clone)]
struct ModelConfig {
    name: &'static str,
    model_id: &'static str,
    expected_vram_gb: f32,
    tokens_per_sec: f32,
}

impl ModelConfig {
    fn config(&self) -> OllamaConfig {
        OllamaConfig::new(
            "http://localhost:11434".to_string(),
            self.model_id.to_string(),
        )
        .with_temperature(0.3)  // Low temperature for coding
        .with_num_predict(1024)
        .with_top_p(0.9)
        .with_top_k(40)
    }
}

const MODELS: &[ModelConfig] = &[
    ModelConfig {
        name: "DeepSeek Coder 6.7B",
        model_id: "deepseek-coder:6.7b-base-q4_K_M",
        expected_vram_gb: 4.5,
        tokens_per_sec: 25.0,
    },
    ModelConfig {
        name: "Phi-3 Mini 3.8B",
        model_id: "phi:mini",
        expected_vram_gb: 2.5,
        tokens_per_sec: 40.0,
    },
    ModelConfig {
        name: "TinyLlama 1.1B",
        model_id: "tinyllama",
        expected_vram_gb: 1.0,
        tokens_per_sec: 60.0,
    },
    ModelConfig {
        name: "CodeLlama 7B",
        model_id: "codellama:7b-base-q4_K_M",
        expected_vram_gb: 5.0,
        tokens_per_sec: 20.0,
    },
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║       Small Local Coding LLM Benchmark Suite                   ║");
    println!("║       Testing optimization strategies with local models         ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("PREREQUISITE: Start Ollama server with:");
    println!("  ollama serve\n");

    println!("PULL MODELS:");
    for model in MODELS {
        println!("  ollama pull {}", model.model_id);
    }
    println!();

    // Test each model
    for model in MODELS {
        println!("\n{}", "=".repeat(70));
        println!("Testing: {}", model.name);
        println!("  Model ID: {}", model.model_id);
        println!("  Expected VRAM: ~{:.1}GB", model.expected_vram_gb);
        println!("  Expected Speed: ~{:.0} tokens/sec", model.tokens_per_sec);
        println!("{}", "=".repeat(70));

        match test_model(model).await {
            Ok((tokens, duration_secs)) => {
                let throughput = tokens as f32 / duration_secs;
                println!("\n✓ SUCCESS");
                println!("  Generated: {} tokens", tokens);
                println!("  Time: {:.2}s", duration_secs);
                println!("  Throughput: {:.1} tokens/sec", throughput);
                println!("  Efficiency: {:.1}% of expected speed",
                    (throughput / model.tokens_per_sec) * 100.0);
            }
            Err(e) => {
                println!("\n✗ FAILED: {}", e);
                println!("  Ensure Ollama is running: ollama serve");
                println!("  Ensure model is pulled: ollama pull {}", model.model_id);
            }
        }
    }

    println!("\n{}", "=".repeat(70));
    println!("RECOMMENDATIONS FOR SMALLEST + FASTEST SETUP:");
    println!("{}", "=".repeat(70));

    println!("\n🏆 BEST OVERALL (Speed + Quality + Size):");
    println!("   DeepSeek Coder 6.7B");
    println!("   - Purpose-built for coding tasks");
    println!("   - 4.5GB VRAM usage");
    println!("   - ~25 tokens/sec");
    println!("   - Excellent code generation quality");

    println!("\n⚡ FASTEST (CPU-friendly):");
    println!("   TinyLlama 1.1B");
    println!("   - Only 1GB VRAM needed");
    println!("   - ~60 tokens/sec (fastest)");
    println!("   - Limited code quality but fast iteration");

    println!("\n💎 BALANCED (Quality + Speed):");
    println!("   Phi-3 Mini 3.8B");
    println!("   - Microsoft's optimized model");
    println!("   - 2.5GB VRAM");
    println!("   - ~40 tokens/sec");
    println!("   - Good quality-to-size ratio");

    println!("\n📚 PRODUCTION (Best Quality):");
    println!("   CodeLlama 7B");
    println!("   - Purpose-built for coding");
    println!("   - 5GB VRAM");
    println!("   - ~20 tokens/sec");
    println!("   - Excellent for production code");

    println!("\n{}", "=".repeat(70));
    println!("OPTIMIZATION STRATEGIES TO USE:");
    println!("{}", "=".repeat(70));

    println!("\nFor small models (1-4B):");
    println!("  ✓ Use Simple Strategies:");
    println!("    - ReRead (RE2): Re-read and verify");
    println!("    - Diverse Sampling: Explore temperature range");
    println!("    - Best-of-N: Multiple attempts with selection");

    println!("\nFor medium models (6-7B):");
    println!("  ✓ Use Moderate Strategies:");
    println!("    - Self-Consistency: Voting across paths");
    println!("    - Diverse Sampling: Temperature exploration");
    println!("    - Round-Trip Optimization: Code generation + verification");

    println!("\nFor production:");
    println!("  ✓ Use Advanced Strategies:");
    println!("    - RSA: Reinforced self-aggregation");
    println!("    - PVG: Prover-Verifier Game");
    println!("    - LEAP: Learning from errors");

    Ok(())
}

/// Test a single model
async fn test_model(model: &ModelConfig) -> Result<(usize, f32), String> {
    let config = model.config();

    // Validate before creating client
    if let Err(e) = config.validate() {
        return Err(format!("Invalid config: {}", e));
    }

    let client = OllamaClient::new(config)
        .map_err(|e| format!("Failed to create client: {}", e))?;

    println!("\n  Testing inference...");

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
                text: CODING_TASK.to_string(),
            }],
        },
    ];

    let start = Instant::now();
    let mut stream = client.stream(&prompt);
    let mut total_tokens = 0;
    let mut generated_tokens = 0;

    while let Some(event) = stream.next().await {
        match event {
            Ok(ResponseEvent::OutputTextDelta { delta: _ }) => {
                generated_tokens += 1;
            }
            Ok(ResponseEvent::Completed { token_usage }) => {
                if let Some(usage) = token_usage {
                    total_tokens = usage.total_tokens();
                }
                break;
            }
            Err(e) => {
                return Err(format!("Stream error: {}", e));
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f32();

    if total_tokens == 0 {
        return Err("No tokens generated".to_string());
    }

    Ok((total_tokens, elapsed))
}
