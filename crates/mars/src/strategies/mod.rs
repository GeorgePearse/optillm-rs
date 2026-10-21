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
//! - pvg: Prover-Verifier Game for adversarial solution generation and verification
//! - leap: Learning from Errors for Adaptive Process using few-shot examples
//! - plansearch: Observation-guided problem solving through planning and implementation
//! - reread: Simple re-reading strategy for improved answer quality
//! - diverse_sampling: Temperature-varied sampling for exploring response space
//! - autothink: Query complexity classification with adaptive reasoning depth
//! - deep_thinking: Inference-time scaling based on problem difficulty
//! - entropy_decoding: Entropy-based sampling for controlled diversity
//! - cot_decoding: Structured chain-of-thought decoding guidance
//! - r_star: Enhanced Monte Carlo Tree Search with learned value estimates

/// AutoThink: Query complexity classification with adaptive reasoning depth
pub mod autothink;
/// Best-of-N sampling strategy
pub mod best_of_n;
/// CoT Decoding: Structured chain-of-thought decoding guidance
pub mod cot_decoding;
/// Chain-of-thought with reflection strategy
pub mod cot_reflection;
/// Deep Thinking: Inference-time scaling based on problem difficulty
pub mod deep_thinking;
/// Diverse sampling with temperature variation
pub mod diverse_sampling;
/// Entropy Decoding: Entropy-based sampling for controlled diversity
pub mod entropy_decoding;
/// Learning from errors adaptive process strategy
pub mod leap;
/// Monte Carlo Tree Search strategy
pub mod mcts;
/// Mixture of Agents strategy
pub mod moa;
/// Plan-guided search strategy
pub mod plansearch;
/// Prover-verifier game strategy
pub mod pvg;
/// R* Algorithm: Enhanced MCTS with learned value estimates
pub mod r_star;
/// Re-reading strategy
pub mod reread;
/// Reinforced self-aggregation strategy
pub mod rsa;
/// Round-trip optimization strategy
pub mod rto;
/// Self-consistency strategy with majority voting
pub mod self_consistency;

// Re-export commonly used types from each strategy
pub use autothink::{
    AutoThinkAggregator, AutoThinkConfig, AutoThinkMetadata, AutoThinkOptimizer, ComplexityLevel,
};
pub use best_of_n::{
    BestOfNAggregator, BestOfNConfig, BestOfNMetadata, SelectionMethod, SelectionStatistics,
};
pub use cot_decoding::{CotDecodingAggregator, CotDecodingConfig, CotDecodingMetadata};
pub use cot_reflection::{CotReflectionAggregator, CotReflectionConfig, CotReflectionMetadata};
pub use deep_thinking::{DeepThinkingAggregator, DeepThinkingConfig, DeepThinkingMetadata};
pub use diverse_sampling::{
    DiverseSamplingAggregator, DiverseSamplingConfig, DiverseSamplingMetadata,
};
pub use entropy_decoding::{
    EntropyDecodingAggregator, EntropyDecodingConfig, EntropyDecodingMetadata,
};
pub use leap::{LEAPAggregator, LEAPConfig, LEAPMetadata};
pub use mcts::{DialogueState, MCTSConfig, MCTSNode, Message, MCTS};
pub use moa::{MoaAggregator, MoaMetadata};
pub use plansearch::{PlanSearchAggregator, PlanSearchConfig, PlanSearchMetadata};
pub use pvg::{PVGAggregator, PVGConfig, PVGMetadata};
pub use r_star::{RStarAggregator, RStarConfig, RStarMetadata};
pub use reread::{ReReadAggregator, ReReadConfig, ReReadMetadata};
pub use rsa::{RSAAggregator, RSAConfig, RSAMetadata, RefinementStrategy, SelectionCriterion};
pub use rto::{RTOAggregator, RTOConfig, RTOMetadata};
pub use self_consistency::{
    AnswerExtractionStrategy, ReasoningPath, SelfConsistencyAggregator, SelfConsistencyConfig,
    SelfConsistencyMetadata, VotingStatistics, VotingStrategy,
};
