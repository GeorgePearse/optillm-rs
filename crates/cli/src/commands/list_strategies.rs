//! List available strategies command

/// List all available strategies
pub fn run() {
    println!("\n=== Available OptimLLM Strategies ===\n");

    let strategies = vec![
        ("AutoThink", "Complexity-aware reasoning with adaptive temperature adjustment"),
        ("Deep Thinking", "Difficulty-based token allocation for variable-complexity problems"),
        ("Entropy Decoding", "Diversity control using Shannon entropy metrics"),
        ("CoT Decoding", "Structured chain-of-thought reasoning patterns"),
        ("R* Algorithm", "Enhanced Monte Carlo Tree Search with learned value estimates"),
        ("MARS", "Multi-agent reasoning system with verification and aggregation"),
    ];

    for (idx, (name, desc)) in strategies.iter().enumerate() {
        println!("{}. {} - {}", idx + 1, name, desc);
    }

    println!("\nUsage: optillm <STRATEGY> [OPTIONS]");
    println!("\nExamples:");
    println!("  optillm autothink --query 'What is 2+2?' --system 'You are helpful.'");
    println!("  optillm deep-thinking --query 'Solve this problem' --min-tokens 256");
    println!("  optillm entropy-decoding --query 'Generate ideas' --target-entropy 0.8");
    println!("\nFor more help: optillm --help");
}
