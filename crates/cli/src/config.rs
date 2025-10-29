//! Configuration management for the CLI

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{CliError, CliResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// API key for the LLM provider
    pub api_key: Option<String>,

    /// LLM model to use
    pub model: Option<String>,

    /// Base URL for the API
    pub api_base: Option<String>,

    /// Default system prompt
    pub system_prompt: Option<String>,

    /// Timeout in seconds
    pub timeout: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            model: None,
            api_base: None,
            system_prompt: None,
            timeout: Some(60),
        }
    }
}

impl Config {
    /// Load configuration from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> CliResult<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| CliError::Config(format!("Failed to read config file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| CliError::Config(format!("Failed to parse config file: {}", e)))
    }

    /// Save configuration to a JSON file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> CliResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| CliError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content)
            .map_err(|e| CliError::Config(format!("Failed to write config file: {}", e)))
    }

    /// Merge another config into this one (other takes precedence)
    pub fn merge(&mut self, other: Config) {
        if other.api_key.is_some() {
            self.api_key = other.api_key;
        }
        if other.model.is_some() {
            self.model = other.model;
        }
        if other.api_base.is_some() {
            self.api_base = other.api_base;
        }
        if other.system_prompt.is_some() {
            self.system_prompt = other.system_prompt;
        }
        if other.timeout.is_some() {
            self.timeout = other.timeout;
        }
    }
}
