//! Example: Using R* Algorithm for intelligent solution space exploration
//!
//! R* enhances MCTS with learned value estimates for smarter exploration.

use optillm_mars::strategies::RStarConfig;

fn main() {
    println!("=== R* Algorithm: Enhanced Monte Carlo Tree Search ===\n");

    // Different configurations for different scenarios
    let fast = RStarConfig {
        num_simulations: 3,
        exploration_constant: 1.0,
        num_candidates: 2,
    };

    let balanced = RStarConfig {
        num_simulations: 10,
        exploration_constant: 1.414,
        num_candidates: 3,
    };

    let thorough = RStarConfig {
        num_simulations: 20,
        exploration_constant: 1.414,
        num_candidates: 5,
    };

    println!("Configuration Profiles:\n");

    println!("SPEED-OPTIMIZED");
    println!("  Simulations: {}", fast.num_simulations);
    println!("  Exploration Constant: {:.3}", fast.exploration_constant);
    println!("  Candidates: {}", fast.num_candidates);
    println!("  Use Case: Fast decisions, time-constrained\n");

    println!("BALANCED (Recommended)");
    println!("  Simulations: {}", balanced.num_simulations);
    println!("  Exploration Constant: {:.3}", balanced.exploration_constant);
    println!("  Candidates: {}", balanced.num_candidates);
    println!("  Use Case: General problem-solving\n");

    println!("QUALITY-OPTIMIZED");
    println!("  Simulations: {}", thorough.num_simulations);
    println!("  Exploration Constant: {:.3}", thorough.exploration_constant);
    println!("  Candidates: {}", thorough.num_candidates);
    println!("  Use Case: Maximum quality, time available\n");

    println!("How R* Works:");
    println!("  1. Initialization: Start with problem");
    println!("  2. Simulation: Run Monte Carlo iterations");
    println!("  3. Selection: Use UCB to pick promising branches");
    println!("  4. Expansion: Generate new solution candidates");
    println!("  5. Evaluation: Score using learned value estimates");
    println!("  6. Learning: Update values based on results");
    println!("  7. Selection: Return best solution found\n");

    println!("Key Advantages:");
    println!("  • Learns what works (value estimates)");
    println!("  • Balances exploration and exploitation (UCB)");
    println!("  • Finds high-quality solutions efficiently");
    println!("  • Transparent decision process");
}
