//! Example: Using Entropy Decoding for controlled diversity
//!
//! Entropy Decoding balances answer quality with diversity using entropy metrics.

use optillm_mars::strategies::EntropyDecodingConfig;

fn main() {
    println!("=== Entropy Decoding: Quality vs Diversity ===\n");

    // Three configurations for different use cases
    let configs = vec![
        ("Factual Questions", EntropyDecodingConfig {
            target_entropy: 0.2,
            entropy_weight: 0.1,
            num_samples: 3,
            min_temperature: 0.3,
            max_temperature: 0.5,
        }),
        ("Mixed Problem", EntropyDecodingConfig {
            target_entropy: 0.5,
            entropy_weight: 0.5,
            num_samples: 3,
            min_temperature: 0.3,
            max_temperature: 1.2,
        }),
        ("Creative Tasks", EntropyDecodingConfig {
            target_entropy: 0.8,
            entropy_weight: 0.8,
            num_samples: 5,
            min_temperature: 0.7,
            max_temperature: 1.5,
        }),
    ];

    for (name, config) in configs {
        println!("Scenario: {}", name);
        println!("  Target Entropy: {:.1}", config.target_entropy);
        println!("  Entropy Weight: {:.1} (0.0=quality, 1.0=diversity)", config.entropy_weight);
        println!("  Samples: {}", config.num_samples);
        println!("  Result: Mix of quality and novelty appropriate for task");
        println!();
    }

    println!("Key Insights:");
    println!("  • Low entropy (0.1-0.3): Focused, consistent answers");
    println!("  • Mid entropy (0.4-0.6): Balanced quality and creativity");
    println!("  • High entropy (0.7-1.0): Diverse, novel perspectives");
}
