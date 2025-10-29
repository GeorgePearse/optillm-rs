//! Example: Using AutoThink strategy for complexity-aware reasoning
//!
//! AutoThink analyzes query complexity and automatically selects optimal model parameters.
//! This example demonstrates how complexity classification affects temperature selection.

use optillm_mars::strategies::{AutoThinkConfig, AutoThinkOptimizer, ComplexityLevel};

fn main() {
    let config = AutoThinkConfig::default();
    let optimizer = AutoThinkOptimizer::new(config);

    // Example queries with different complexities
    let queries = vec![
        ("What is the capital of France?", ComplexityLevel::Simple),
        ("Explain how binary search trees work and analyze their time complexity", ComplexityLevel::Medium),
        ("Prove that the set of all prime numbers is infinite using contradiction", ComplexityLevel::Complex),
    ];

    println!("=== AutoThink Complexity Classification ===\n");

    for (query, expected_level) in queries {
        let complexity = optimizer.classify_complexity(query);
        let score = optimizer.classify_complexity(query);
        let temperature = optimizer.get_temperature(&complexity);

        println!("Query: \"{}\"", query);
        println!("Expected Complexity: {:?}", expected_level);
        println!("Detected Complexity: {:?}", complexity);
        println!("Temperature Selected: {:.1}", temperature);
        println!("Reasoning: ", );
        match complexity {
            ComplexityLevel::Simple => {
                println!("  → Low temperature (0.3) for focused, deterministic response");
            }
            ComplexityLevel::Medium => {
                println!("  → Medium temperature (0.6) for balanced reasoning");
            }
            ComplexityLevel::Complex => {
                println!("  → High temperature (1.0) for exploratory reasoning");
            }
        }
        println!();
    }

    // Show difficulty score breakdown
    println!("=== Difficulty Score Analysis ===\n");

    let complex_query = "Derive the Fourier transform from first principles and explain its applications in signal processing";
    let score = 0.78; // Would be calculated by optimizer
    println!("Query: \"{}\"", complex_query);
    println!("Complexity Score: {:.2}", score);
    println!("Score Interpretation:");
    println!("  - 0.0-0.33: Simple → Temperature 0.3");
    println!("  - 0.33-0.67: Medium → Temperature 0.6");
    println!("  - 0.67-1.0: Complex → Temperature 1.0");
    println!();
    println!("This query scores {:.2} → Complex (temperature 1.0)", score);
}
