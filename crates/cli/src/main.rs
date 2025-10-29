//! OptimLLM CLI - Command-line interface for LLM optimization strategies
//!
//! Provides a unified CLI for integrating OptimLLM strategies into any LLM workflow.

use clap::{Parser, Subcommand};
use std::env;
use tracing_subscriber;

mod commands;
mod config;
mod error;

use error::CliResult;

#[derive(Parser)]
#[command(
    name = "optillm",
    version = "0.1.0",
    about = "OptimLLM: LLM Optimization Strategies CLI",
    long_about = "A command-line interface for integrating advanced LLM optimization strategies into any coding or reasoning system."
)]
struct Cli {
    /// Enable verbose logging output
    #[arg(global = true, short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable colored output
    #[arg(global = true, long)]
    no_color: bool,

    /// Configuration file path (JSON format)
    #[arg(global = true, short, long, env = "OPTILLM_CONFIG")]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run AutoThink strategy (complexity-aware reasoning)
    AutoThink {
        /// Query or problem to solve
        #[arg(short, long)]
        query: String,

        /// System prompt for the model
        #[arg(short, long, default_value = "You are a helpful assistant.")]
        system: String,

        /// Complexity threshold for medium difficulty (0.0-1.0)
        #[arg(long, default_value = "0.25")]
        simple_threshold: f32,

        /// Complexity threshold for complex difficulty (0.0-1.0)
        #[arg(long, default_value = "0.40")]
        complex_threshold: f32,
    },

    /// Run Deep Thinking strategy (difficulty-based token allocation)
    DeepThinking {
        /// Query or problem to solve
        #[arg(short, long)]
        query: String,

        /// System prompt for the model
        #[arg(short, long, default_value = "You are a helpful assistant.")]
        system: String,

        /// Minimum token allocation
        #[arg(long, default_value = "256")]
        min_tokens: usize,

        /// Maximum token allocation
        #[arg(long, default_value = "2048")]
        max_tokens: usize,

        /// Number of reasoning iterations
        #[arg(long, default_value = "3")]
        iterations: usize,
    },

    /// Run Entropy Decoding strategy (diversity control)
    EntropyDecoding {
        /// Query or problem to solve
        #[arg(short, long)]
        query: String,

        /// System prompt for the model
        #[arg(short, long, default_value = "You are a helpful assistant.")]
        system: String,

        /// Target entropy level (0.0-1.0)
        #[arg(long, default_value = "0.6")]
        target_entropy: f32,

        /// Number of samples to generate
        #[arg(long, default_value = "3")]
        num_samples: usize,
    },

    /// Run CoT Decoding strategy (structured reasoning)
    CotDecoding {
        /// Query or problem to solve
        #[arg(short, long)]
        query: String,

        /// System prompt for the model
        #[arg(short, long, default_value = "You are a helpful assistant.")]
        system: String,

        /// Number of reasoning steps
        #[arg(long, default_value = "4")]
        steps: usize,

        /// Enable verification of reasoning
        #[arg(long)]
        verify: bool,
    },

    /// Run R* Algorithm strategy (enhanced tree search)
    RStar {
        /// Query or problem to solve
        #[arg(short, long)]
        query: String,

        /// System prompt for the model
        #[arg(short, long, default_value = "You are a helpful assistant.")]
        system: String,

        /// Number of MCTS simulations
        #[arg(long, default_value = "10")]
        simulations: usize,

        /// UCB exploration constant
        #[arg(long, default_value = "1.414")]
        exploration: f32,

        /// Number of candidate solutions
        #[arg(long, default_value = "3")]
        candidates: usize,
    },

    /// Run MARS strategy (multi-agent reasoning)
    Mars {
        /// Query or problem to solve
        #[arg(short, long)]
        query: String,

        /// System prompt for the model
        #[arg(short, long, default_value = "You are a helpful assistant.")]
        system: String,

        /// Number of agents to use
        #[arg(long, default_value = "3")]
        num_agents: usize,
    },

    /// List all available strategies
    Strategies,
}

#[tokio::main]
async fn main() -> CliResult<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.no_color)?;

    tracing::info!("OptimLLM CLI v{}", env!("CARGO_PKG_VERSION"));

    // Route to appropriate command handler
    match cli.command {
        Commands::AutoThink {
            query,
            system,
            simple_threshold,
            complex_threshold,
        } => {
            commands::autothink::run(query, system, simple_threshold, complex_threshold).await?;
        }

        Commands::DeepThinking {
            query,
            system,
            min_tokens,
            max_tokens,
            iterations,
        } => {
            commands::deep_thinking::run(query, system, min_tokens, max_tokens, iterations).await?;
        }

        Commands::EntropyDecoding {
            query,
            system,
            target_entropy,
            num_samples,
        } => {
            commands::entropy_decoding::run(query, system, target_entropy, num_samples).await?;
        }

        Commands::CotDecoding {
            query,
            system,
            steps,
            verify,
        } => {
            commands::cot_decoding::run(query, system, steps, verify).await?;
        }

        Commands::RStar {
            query,
            system,
            simulations,
            exploration,
            candidates,
        } => {
            commands::r_star::run(query, system, simulations, exploration, candidates).await?;
        }

        Commands::Mars {
            query,
            system,
            num_agents,
        } => {
            commands::mars::run(query, system, num_agents).await?;
        }

        Commands::Strategies => {
            commands::list_strategies::run();
        }
    }

    Ok(())
}

fn init_logging(verbose: u8, no_color: bool) -> CliResult<()> {
    let log_level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(log_level))?;

    let builder = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_ansi(!no_color);

    builder.init();

    Ok(())
}
