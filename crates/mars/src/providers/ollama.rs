//! Ollama local model inference provider
//!
//! Provides integration with Ollama for running local language models.
//! Ollama is a simple, user-friendly tool for running open-source LLMs locally.
//!
//! # Example
//! ```ignore
//! use optillm_mars::providers::ollama::{OllamaClient, OllamaConfig};
//! use optillm_core::ModelClient;
//!
//! let config = OllamaConfig::new("http://localhost:11434".to_string(), "llama2".to_string());
//! let client = OllamaClient::new(config)?;
//! ```

use serde::{Deserialize, Serialize};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

use optillm_core::{ModelClient, Prompt, ResponseEvent, ResponseItem, ContentItem, OptillmError};

/// Configuration for Ollama client
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OllamaConfig {
    /// Base URL of Ollama server (e.g., "http://localhost:11434")
    pub base_url: String,
    /// Model name to use (e.g., "llama2", "mistral", "neural-chat")
    pub model: String,
    /// Temperature for response generation (0.0 to 2.0)
    pub temperature: f32,
    /// Maximum tokens to generate
    pub num_predict: usize,
    /// Top-p (nucleus) sampling parameter
    pub top_p: f32,
    /// Top-k sampling parameter
    pub top_k: usize,
}

impl OllamaConfig {
    /// Create new Ollama configuration with required parameters
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            temperature: 0.7,
            num_predict: 4096,
            top_p: 0.9,
            top_k: 40,
        }
    }

    /// Set temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    /// Set maximum tokens to predict
    pub fn with_num_predict(mut self, tokens: usize) -> Self {
        self.num_predict = tokens;
        self
    }

    /// Set top-p sampling parameter
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p;
        self
    }

    /// Set top-k sampling parameter
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k;
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> std::result::Result<(), OptillmError> {
        if self.base_url.is_empty() {
            return Err(OptillmError::InvalidConfiguration(
                "base_url cannot be empty".to_string(),
            ));
        }
        if self.model.is_empty() {
            return Err(OptillmError::InvalidConfiguration(
                "model cannot be empty".to_string(),
            ));
        }
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(OptillmError::InvalidConfiguration(
                "temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
        if self.num_predict == 0 {
            return Err(OptillmError::InvalidConfiguration(
                "num_predict must be greater than 0".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&self.top_p) {
            return Err(OptillmError::InvalidConfiguration(
                "top_p must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "llama2".to_string(),
            temperature: 0.7,
            num_predict: 4096,
            top_p: 0.9,
            top_k: 40,
        }
    }
}

/// Request payload for Ollama API
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    temperature: f32,
    num_predict: usize,
    top_p: f32,
    top_k: usize,
}

/// Message for Ollama API
#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Response from Ollama API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaResponse {
    model: String,
    created_at: String,
    message: Option<OllamaMessage>,
    done: bool,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<usize>,
    prompt_eval_duration: Option<u64>,
    eval_count: Option<usize>,
    eval_duration: Option<u64>,
}

/// Ollama local model inference client
pub struct OllamaClient {
    config: OllamaConfig,
    http_client: reqwest::Client,
}

impl OllamaClient {
    /// Create new Ollama client
    pub fn new(config: OllamaConfig) -> std::result::Result<Self, OptillmError> {
        config.validate()?;
        Ok(Self {
            config,
            http_client: reqwest::Client::new(),
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &OllamaConfig {
        &self.config
    }

    /// Build messages from prompt
    fn build_messages(prompt: &Prompt) -> Vec<OllamaMessage> {
        let mut messages = Vec::new();

        for item in &prompt.input {
            match item {
                ResponseItem::Message { role, content, .. } => {
                    let text_content = content
                        .iter()
                        .filter_map(|c| match c {
                            ContentItem::InputText { text } => Some(text.clone()),
                            ContentItem::Text { text } => Some(text.clone()),
                        })
                        .collect::<Vec<_>>()
                        .join("");

                    if !text_content.is_empty() {
                        messages.push(OllamaMessage {
                            role: role.clone(),
                            content: text_content,
                        });
                    }
                }
            }
        }

        messages
    }
}

impl ModelClient for OllamaClient {
    fn stream(
        &self,
        prompt: &Prompt,
    ) -> Pin<Box<dyn Stream<Item = optillm_core::Result<ResponseEvent>> + Send>> {
        let config = self.config.clone();
        let http_client = self.http_client.clone();
        let messages = Self::build_messages(prompt);

        Box::pin(async_stream::stream! {
            let url = format!("{}/api/chat", config.base_url);

            let request = OllamaRequest {
                model: config.model.clone(),
                messages,
                stream: true,
                temperature: config.temperature,
                num_predict: config.num_predict,
                top_p: config.top_p,
                top_k: config.top_k,
            };

            let response = match http_client
                .post(&url)
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    yield Err(OptillmError::ClientError(format!("Failed to connect to Ollama: {}", e)));
                    return;
                }
            };

            if !response.status().is_success() {
                yield Err(OptillmError::ClientError(format!(
                    "Ollama API error: {}",
                    response.status()
                )));
                return;
            }

            let mut bytes_stream = response.bytes_stream();
            let mut buffered = String::new();
            let mut total_eval_count = 0;
            let mut prompt_eval_count = 0;

            while let Some(result) = bytes_stream.next().await {
                let chunk = match result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(OptillmError::ClientError(format!(
                            "Failed to read response stream: {}",
                            e
                        )));
                        return;
                    }
                };

                buffered.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete lines
                while let Some(newline_pos) = buffered.find('\n') {
                    let line = buffered[..newline_pos].to_string();
                    buffered = buffered[newline_pos + 1..].to_string();

                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<OllamaResponse>(line.trim()) {
                        Ok(ollama_resp) => {
                            // Extract text delta if present
                            if let Some(msg) = &ollama_resp.message {
                                if !msg.content.is_empty() {
                                    yield Ok(ResponseEvent::OutputTextDelta {
                                        delta: msg.content.clone(),
                                    });
                                }
                            }

                            // Track token counts from final response
                            if ollama_resp.done {
                                if let Some(eval_count) = ollama_resp.eval_count {
                                    total_eval_count = eval_count;
                                }
                                if let Some(prompt_count) = ollama_resp.prompt_eval_count {
                                    prompt_eval_count = prompt_count;
                                }

                                // Send completion event
                                let token_usage = optillm_core::TokenUsage {
                                    input_tokens: prompt_eval_count,
                                    output_tokens: total_eval_count,
                                };

                                yield Ok(ResponseEvent::Completed {
                                    token_usage: Some(token_usage),
                                });
                            }
                        }
                        Err(e) => {
                            yield Err(OptillmError::ParsingError(format!(
                                "Failed to parse Ollama response: {}",
                                e
                            )));
                            return;
                        }
                    }
                }
            }

            // Process any remaining buffered data
            if !buffered.trim().is_empty() {
                match serde_json::from_str::<OllamaResponse>(buffered.trim()) {
                    Ok(ollama_resp) => {
                        if let Some(msg) = &ollama_resp.message {
                            if !msg.content.is_empty() {
                                yield Ok(ResponseEvent::OutputTextDelta {
                                    delta: msg.content.clone(),
                                });
                            }
                        }

                        if let Some(eval_count) = ollama_resp.eval_count {
                            total_eval_count = eval_count;
                        }
                        if let Some(prompt_count) = ollama_resp.prompt_eval_count {
                            prompt_eval_count = prompt_count;
                        }

                        let token_usage = optillm_core::TokenUsage {
                            input_tokens: prompt_eval_count,
                            output_tokens: total_eval_count,
                        };

                        yield Ok(ResponseEvent::Completed {
                            token_usage: Some(token_usage),
                        });
                    }
                    Err(e) => {
                        yield Err(OptillmError::ParsingError(format!(
                            "Failed to parse Ollama response: {}",
                            e
                        )));
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config_new() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        );
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "llama2");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.num_predict, 4096);
    }

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "llama2");
    }

    #[test]
    fn test_ollama_config_builder() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "mistral".to_string(),
        )
        .with_temperature(0.5)
        .with_num_predict(2048)
        .with_top_p(0.8)
        .with_top_k(30);

        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.num_predict, 2048);
        assert_eq!(config.top_p, 0.8);
        assert_eq!(config.top_k, 30);
    }

    #[test]
    fn test_ollama_config_validation_valid() {
        let config = OllamaConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_config_validation_empty_base_url() {
        let config = OllamaConfig::new(String::new(), "llama2".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_validation_empty_model() {
        let config = OllamaConfig::new("http://localhost:11434".to_string(), String::new());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_validation_temperature_too_low() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_temperature(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_validation_temperature_too_high() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_temperature(2.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_validation_zero_tokens() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_num_predict(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_validation_invalid_top_p_too_high() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_top_p(1.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_validation_invalid_top_p_negative() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_top_p(-0.1);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_validation_temperature_boundary_low() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_temperature(0.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_config_validation_temperature_boundary_high() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_temperature(2.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_config_validation_top_p_boundary_zero() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_top_p(0.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_config_validation_top_p_boundary_one() {
        let config = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "llama2".to_string(),
        )
        .with_top_p(1.0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ollama_client_creation() {
        let config = OllamaConfig::default();
        let client = OllamaClient::new(config.clone());
        assert!(client.is_ok());
        if let Ok(client) = client {
            assert_eq!(client.config(), &config);
        }
    }

    #[test]
    fn test_ollama_client_creation_fails_with_invalid_config() {
        let config = OllamaConfig::new(String::new(), "llama2".to_string());
        let client = OllamaClient::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn test_ollama_config_equality() {
        let config1 = OllamaConfig::default();
        let config2 = OllamaConfig::default();
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_ollama_config_inequality() {
        let config1 = OllamaConfig::default();
        let config2 = OllamaConfig::new(
            "http://localhost:11434".to_string(),
            "mistral".to_string(),
        );
        assert_ne!(config1, config2);
    }

    #[test]
    fn test_ollama_message_serialization() {
        let msg = OllamaMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_ollama_request_serialization() {
        let request = OllamaRequest {
            model: "llama2".to_string(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: "Test".to_string(),
            }],
            stream: true,
            temperature: 0.7,
            num_predict: 4096,
            top_p: 0.9,
            top_k: 40,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("llama2"));
        assert!(json.contains("true")); // stream: true
    }
}
