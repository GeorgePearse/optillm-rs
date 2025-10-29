/// Model router for unified access to multiple LLM providers.
///
/// Provides abstraction layer supporting both optillm_core::ModelClient
/// and alternative providers for flexible provider selection.

use crate::Result;

/// Stream wrapper for generic model responses
#[derive(Clone, Debug)]
pub struct ModelStream {
    /// Response content
    pub content: String,
    /// Current position in stream
    position: usize,
}

impl ModelStream {
    /// Create new model stream from content
    pub fn new(content: String) -> Self {
        Self {
            content,
            position: 0,
        }
    }

    /// Get next chunk of streaming content
    pub fn next_chunk(&mut self) -> Option<String> {
        if self.position >= self.content.len() {
            return None;
        }

        // For now, yield the entire content
        // In production, this would stream incrementally
        let chunk = self.content.clone();
        self.position = self.content.len();
        Some(chunk)
    }
}

/// Generic LLM provider trait for unified provider access
pub trait LLMProvider: Send + Sync {
    /// Complete a prompt and return the full response
    fn complete_blocking(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String>;

    /// Get provider name for logging/debugging
    fn provider_name(&self) -> &str;

    /// Get model name for logging/debugging
    fn model_name(&self) -> &str;
}

/// Wrapper around ModelClient for backward compatibility with optillm_core
pub struct ModelClientRouter {
    /// Placeholder for ModelClient integration
    /// Full implementation would store a Box<dyn ModelClient> here
    _marker: std::marker::PhantomData<()>,
}

impl ModelClientRouter {
    /// Create new ModelClient router
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl LLMProvider for ModelClientRouter {
    fn complete_blocking(&self, prompt: &str, system_prompt: Option<&str>) -> Result<String> {
        // Since ModelClient is async, we cannot implement a blocking version directly
        // This demonstrates the interface contract while full async support
        // can be added to the optimization pipeline
        let user_prompt = if let Some(system) = system_prompt {
            format!("{}\n\n{}", system, prompt)
        } else {
            prompt.to_string()
        };

        // For now, return a placeholder
        // Full implementation would require async context in the optimizer trait
        Ok(format!("[Response to: {}]", user_prompt))
    }

    fn provider_name(&self) -> &str {
        "optillm-client"
    }

    fn model_name(&self) -> &str {
        "optillm-model"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_stream_creation() {
        let stream = ModelStream::new("Hello, world!".to_string());
        assert_eq!(stream.content, "Hello, world!");
        assert_eq!(stream.position, 0);
    }

    #[test]
    fn test_model_stream_next_chunk() {
        let mut stream = ModelStream::new("Hello, world!".to_string());
        let chunk = stream.next_chunk();
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap(), "Hello, world!");

        let chunk2 = stream.next_chunk();
        assert!(chunk2.is_none());
    }
}
