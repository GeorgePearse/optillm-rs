//! Example: Using CoT Decoding for structured reasoning
//!
//! CoT Decoding guides models through structured problem-solving frameworks.

use optillm_mars::strategies::CotDecodingConfig;

fn main() {
    println!("=== CoT Decoding: Structured Reasoning Patterns ===\n");

    println!("Available Reasoning Structures:\n");

    println!("1. LINEAR STRUCTURE");
    println!("   Best for: Sequential reasoning, proofs, algorithms");
    println!("   Example:");
    println!("     Step 1: Define the problem");
    println!("     Step 2: Identify key components");
    println!("     Step 3: Develop solution");
    println!("     Step 4: Verify correctness\n");

    println!("2. TREE STRUCTURE");
    println!("   Best for: Comparative analysis, design decisions");
    println!("   Example:");
    println!("     Approach A: [First alternative]");
    println!("     Approach B: [Second alternative]");
    println!("     Approach C: [Third alternative]");
    println!("     Best: [Selection and justification]\n");

    println!("3. DIALOGUE STRUCTURE");
    println!("   Best for: Exploratory reasoning, Q&A style");
    println!("   Example:");
    println!("     Question: [Problem statement]");
    println!("     Observation 1: [First insight]");
    println!("     Observation 2: [Second insight]");
    println!("     Verification: [Check logic]\n");

    println!("4. ANALYSIS-SYNTHESIS STRUCTURE");
    println!("   Best for: Complex systems, integration");
    println!("   Example:");
    println!("     Analysis:");
    println!("       - Component 1: [Details]");
    println!("       - Component 2: [Details]");
    println!("     Synthesis: [How they connect]\n");

    let config = CotDecodingConfig::default();
    println!("Configuration:");
    println!("  Number of Steps: {}", config.num_steps);
    println!("  Verification Enabled: {}", config.enable_verification);
}
