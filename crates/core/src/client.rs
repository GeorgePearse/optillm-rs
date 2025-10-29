//! Model client interfaces for LLM communication.

use serde::{Deserialize, Serialize};
use futures::stream::Stream;

/// Represents a prompt for the model client
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Prompt {
    /// Messages in this prompt
    pub input: Vec<ResponseItem>,
    /// Optional system instructions override
    pub base_instructions_override: Option<String>,
    /// Optional tag for logging/tracing
    #[serde(skip)]
    pub log_tag: Option<String>,
}

/// Items in a response/prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseItem {
    /// A message with role and content
    Message {
        /// Optional message ID
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Message role (e.g., "user", "assistant")
        role: String,
        /// Content items in the message
        content: Vec<ContentItem>,
    },
}

/// Content items in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentItem {
    /// Plain text input
    InputText {
        /// The text content
        text: String
    },
    /// Plain text output
    Text {
        /// The text content
        text: String
    },
}

/// Events from model responses
#[derive(Debug, Clone)]
pub enum ResponseEvent {
    /// Text delta in streaming response
    OutputTextDelta {
        /// The text delta
        delta: String
    },
    /// Completion event with token usage
    Completed {
        /// Token usage information
        token_usage: Option<TokenUsage>
    },
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input tokens used
    pub input_tokens: usize,
    /// Number of output tokens generated
    pub output_tokens: usize,
}

impl TokenUsage {
    /// Total number of tokens used
    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }
}

impl Prompt {
    /// Create a new empty prompt
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the log tag for this prompt
    pub fn set_log_tag(&mut self, tag: &str) {
        self.log_tag = Some(tag.to_string());
    }
}

/// A client for making requests to a language model
///
/// This trait can be implemented with various backends
/// (e.g., HTTP API clients, local models, etc.)
pub trait ModelClient: Send + Sync {
    /// Stream a completion response
    ///
    /// Returns a boxed stream of response events
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> std::pin::Pin<Box<dyn Stream<Item = crate::Result<ResponseEvent>> + Send>>;
}
