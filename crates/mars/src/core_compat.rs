//! Backward compatibility module for code_core references.
//!
//! This module provides aliases to optillm_core types to maintain compatibility
//! with code that was written to reference code_core directly.

pub use crate::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem, TokenUsage};

/// Re-export as code_core for code that uses that module name
pub mod code_core {
    /// Re-exported core types for compatibility
    pub use super::{ContentItem, ModelClient, Prompt, ResponseEvent, ResponseItem, TokenUsage};
}
