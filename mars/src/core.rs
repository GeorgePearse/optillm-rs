/// Stub/adapter module for code_core types
/// This provides minimal implementations needed for MARS to function standalone

use serde::{Deserialize, Serialize};
use futures::stream::Stream;

/// Represents a prompt for the model client
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Prompt {
    pub input: Vec<ResponseItem>,
    pub base_instructions_override: Option<String>,
    #[serde(skip)]
    pub log_tag: Option<String>,
}

/// Items in a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseItem {
    Message {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        role: String,
        content: Vec<ContentItem>,
    },
}

/// Content items in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentItem {
    InputText { text: String },
    Text { text: String },
}

/// Events from model responses
#[derive(Debug, Clone)]
pub enum ResponseEvent {
    OutputTextDelta { delta: String },
    Completed { token_usage: Option<TokenUsage> },
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl TokenUsage {
    /// Total number of tokens used
    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }
}

/// A client for making requests to a language model
///
/// This is a trait that can be implemented with various backends
/// (e.g., HTTP API clients, local models, etc.)
pub trait ModelClient: Send + Sync {
    /// Stream a completion response
    /// Returns a boxed stream of response events
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> std::pin::Pin<Box<dyn Stream<Item = Result<ResponseEvent, Box<dyn std::error::Error>>> + Send>>;
}

impl Prompt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_log_tag(&mut self, tag: &str) {
        self.log_tag = Some(tag.to_string());
    }
}
