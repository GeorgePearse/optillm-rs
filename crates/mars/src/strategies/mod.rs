//! Strategy implementations for MARS optimization
//!
//! This module contains various LLM optimization strategies:
//! - best_of_n: Generate N solutions and select the best
//! - self_consistency: Consensus voting across diverse reasoning paths
//! - rsa: Reinforced self-aggregation with iterative refinement
//! - mcts: Monte Carlo Tree Search for systematic exploration
//! - moa: Mixture of Agents leveraging multiple models
//! - cot_reflection: Chain-of-thought with self-reflection and refinement
//! - rto: Round Trip Optimization for quality improvement through round-trip generation

pub mod best_of_n;
pub mod self_consistency;
pub mod rsa;
pub mod mcts;
pub mod moa;
pub mod cot_reflection;
pub mod rto;

// Re-export commonly used types from each strategy
pub use best_of_n::{BestOfNAggregator, BestOfNConfig, BestOfNMetadata, SelectionMethod, SelectionStatistics};
pub use self_consistency::{
    SelfConsistencyAggregator, SelfConsistencyConfig, SelfConsistencyMetadata,
    AnswerExtractionStrategy, VotingStrategy, ReasoningPath, VotingStatistics,
};
pub use rsa::{RSAAggregator, RSAConfig, RSAMetadata, SelectionCriterion, RefinementStrategy};
pub use mcts::{MCTS, MCTSConfig, MCTSNode, DialogueState, Message};
pub use moa::{MoaAggregator, MoaMetadata};
pub use cot_reflection::{CotReflectionAggregator, CotReflectionConfig, CotReflectionMetadata};
pub use rto::{RTOAggregator, RTOConfig, RTOMetadata};
