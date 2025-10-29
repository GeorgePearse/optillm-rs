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

/// Best-of-N sampling strategy
pub mod best_of_n;
/// AutoThink: Query complexity classification with adaptive reasoning depth
pub mod autothink;
/// Deep Thinking: Inference-time scaling based on problem difficulty
pub mod deep_thinking;
/// Entropy Decoding: Entropy-based sampling for controlled diversity
pub mod entropy_decoding;
/// CoT Decoding: Structured chain-of-thought decoding guidance
pub mod cot_decoding;
/// R* Algorithm: Enhanced MCTS with learned value estimates
pub mod r_star;
/// Self-consistency strategy with majority voting
pub mod self_consistency;
/// Reinforced self-aggregation strategy
pub mod rsa;
/// Monte Carlo Tree Search strategy
pub mod mcts;
/// Mixture of Agents strategy
pub mod moa;
/// Chain-of-thought with reflection strategy
pub mod cot_reflection;
/// Round-trip optimization strategy
pub mod rto;
/// Prover-verifier game strategy
pub mod pvg;
/// Learning from errors adaptive process strategy
pub mod leap;
/// Plan-guided search strategy
pub mod plansearch;
/// Re-reading strategy
pub mod reread;
/// Diverse sampling with temperature variation
pub mod diverse_sampling;

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
pub use pvg::{PVGAggregator, PVGConfig, PVGMetadata};
pub use leap::{LEAPAggregator, LEAPConfig, LEAPMetadata};
pub use plansearch::{PlanSearchAggregator, PlanSearchConfig, PlanSearchMetadata};
pub use reread::{ReReadAggregator, ReReadConfig, ReReadMetadata};
pub use diverse_sampling::{DiverseSamplingAggregator, DiverseSamplingConfig, DiverseSamplingMetadata};
pub use autothink::{AutoThinkAggregator, AutoThinkConfig, ComplexityLevel, AutoThinkMetadata, AutoThinkOptimizer};
pub use deep_thinking::{DeepThinkingAggregator, DeepThinkingConfig, DeepThinkingMetadata};
pub use entropy_decoding::{EntropyDecodingAggregator, EntropyDecodingConfig, EntropyDecodingMetadata};
pub use cot_decoding::{CotDecodingAggregator, CotDecodingConfig, CotDecodingMetadata};
pub use r_star::{RStarAggregator, RStarConfig, RStarMetadata};
