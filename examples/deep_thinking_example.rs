//! Example: Using Deep Thinking for difficulty-based token allocation
//!
//! Deep Thinking allocates more computation (tokens) to harder problems.

use optillm_mars::strategies::DeepThinkingConfig;

fn main() {
    println!("=== Deep Thinking Token Allocation ===\n");

    let config = DeepThinkingConfig {
        min_tokens: 256,
        max_tokens: 2048,
        num_iterations: 3,
    };

    // Simulate difficulty estimates and token allocation
    let problems = vec![
        ("What is 2+2?", 0.1),
        ("Analyze quicksort time complexity", 0.5),
        ("Design distributed consensus algorithm", 0.9),
    ];

    for (problem, difficulty) in problems {
        let tokens = config.min_tokens as f32
            + difficulty * (config.max_tokens as f32 - config.min_tokens as f32);

        println!("Problem: \"{}\"", problem);
        println!("Difficulty: {:.1}", difficulty);
        println!("Token Budget: {:.0} tokens", tokens);
        println!("Iterations: {}", config.num_iterations);
        println!();
    }

    println!("Benefits:");
    println!("  • Simple problems get quick responses (~300 tokens)");
    println!("  • Complex problems get full reasoning (~1900 tokens)");
    println!("  • Efficiency: Save computation on easy problems");
    println!("  • Quality: Better answers for hard problems");
}
