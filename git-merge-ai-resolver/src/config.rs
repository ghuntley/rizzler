// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

/// Configuration for the git-merge-ai-resolver
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// AI provider configuration
    #[serde(default)]
    pub ai_provider: AIProviderConfig,
    
    /// Resolution strategies configuration
    #[serde(default)]
    pub resolution: ResolutionConfig,
    
    /// Git integration configuration
    #[serde(default)]
    pub git: GitConfig,
    
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// AI provider configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AIProviderConfig {
    /// Default AI provider to use
    pub default_provider: Option<String>,
    
    /// Default AI model to use
    pub default_model: Option<String>,
    
    /// Custom system prompt for AI
    pub system_prompt: Option<String>,
    
    /// Timeout for AI requests in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

/// Resolution strategies configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ResolutionConfig {
    /// Default resolution strategy
    #[serde(default = "default_strategy")]
    pub default_strategy: String,
    
    /// File extension to strategy mappings
    /// Maps file extensions to resolution strategies (e.g., "js" -> "ai", "md" -> "simple")
    #[serde(default)]
    pub extension_strategies: HashMap<String, String>,
}

/// Git integration configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GitConfig {
    /// Associated file extensions
    #[serde(default)]
    pub file_extensions: Vec<String>,
}

/// Logging configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    #[serde(default = "default_log_level")]
    pub level: String,
    
    /// Path to log file
    pub file: Option<String>,
}

// Default values for configuration
fn default_timeout() -> u64 { 30 }
fn default_strategy() -> String { "ai".to_string() }
fn default_log_level() -> String { "info".to_string() }

/// Error types for configuration operations
#[derive(Debug)]
pub enum ConfigError {
    /// IO error
    IoError(std::io::Error),
    
    /// Parse error
    ParseError(String),
    
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IO error: {}", err),
            Self::ParseError(err) => write!(f, "Parse error: {}", err),
            Self::InvalidConfig(err) => write!(f, "Invalid configuration: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::ParseError(err.to_string())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ai_provider: AIProviderConfig::default(),
            resolution: ResolutionConfig::default(),
            git: GitConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables and Git config
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Config::default();
        
        // Load environment variables
        config.load_from_env();
        
        // TODO: Load from Git config
        
        Ok(config)
    }
    
    /// Get the resolution strategy for a specific file
    /// 
    /// Returns the strategy name for the file based on its extension, or the default strategy if no
    /// extension-specific strategy is configured
    pub fn get_strategy_for_file(&self, file_path: &str) -> &str {
        // Extract the file extension
        let extension = Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        // Look up the strategy for this extension, or use the default
        self.resolution.extension_strategies
            .get(extension)
            .map(|s| s.as_str())
            .unwrap_or(&self.resolution.default_strategy)
    }
    
    /// Load configuration from environment variables
    fn load_from_env(&mut self) {
        // AI provider configuration
        if let Ok(provider) = env::var("GIT_MERGE_AI_PROVIDER") {
            self.ai_provider.default_provider = Some(provider);
        }
        
        if let Ok(model) = env::var("GIT_MERGE_AI_MODEL") {
            self.ai_provider.default_model = Some(model);
        }
        
        if let Ok(prompt) = env::var("GIT_MERGE_AI_SYSTEM_PROMPT") {
            self.ai_provider.system_prompt = Some(prompt);
        }
        
        if let Ok(timeout) = env::var("GIT_MERGE_AI_TIMEOUT") {
            if let Ok(timeout) = timeout.parse::<u64>() {
                self.ai_provider.timeout_seconds = timeout;
            }
        }
        
        // Logging configuration
        if let Ok(level) = env::var("GIT_MERGE_LOG_LEVEL") {
            self.logging.level = level;
        }
        
        if let Ok(file) = env::var("GIT_MERGE_LOG_FILE") {
            self.logging.file = Some(file);
        }
        
        // Resolution configuration
        if let Ok(strategy) = env::var("GIT_MERGE_DEFAULT_STRATEGY") {
            self.resolution.default_strategy = strategy;
        }
        
        // Load file extension strategies from environment variables
        // Format: GIT_MERGE_EXTENSION_STRATEGY_<extension>=<strategy>
        for (key, value) in env::vars() {
            if key.starts_with("GIT_MERGE_EXTENSION_STRATEGY_") {
                if let Some(extension) = key.strip_prefix("GIT_MERGE_EXTENSION_STRATEGY_") {
                    let strategy = value.clone(); // Clone to avoid moved value error
                    self.resolution.extension_strategies.insert(extension.to_string(), strategy.clone());
                    debug!("Added extension strategy mapping: {} -> {}", extension, strategy);
                }
            }
        }
    }
    
    /// Get a configuration value by key
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "ai_provider.default_provider" => self.ai_provider.default_provider.clone(),
            "ai_provider.default_model" => self.ai_provider.default_model.clone(),
            "ai_provider.system_prompt" => self.ai_provider.system_prompt.clone(),
            "ai_provider.timeout_seconds" => Some(self.ai_provider.timeout_seconds.to_string()),
            "resolution.default_strategy" => Some(self.resolution.default_strategy.clone()),
            "logging.level" => Some(self.logging.level.clone()),
            "logging.file" => self.logging.file.clone(),
            _ => None,
        }
    }
    
    /// Set a configuration value by key
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "ai_provider.default_provider" => self.ai_provider.default_provider = Some(value.to_string()),
            "ai_provider.default_model" => self.ai_provider.default_model = Some(value.to_string()),
            "ai_provider.system_prompt" => self.ai_provider.system_prompt = Some(value.to_string()),
            "ai_provider.timeout_seconds" => {
                if let Ok(timeout) = value.parse::<u64>() {
                    self.ai_provider.timeout_seconds = timeout;
                } else {
                    return Err(ConfigError::InvalidConfig(format!("Invalid timeout value: {}", value)));
                }
            },
            "resolution.default_strategy" => self.resolution.default_strategy = value.to_string(),
            "logging.level" => self.logging.level = value.to_string(),
            "logging.file" => self.logging.file = Some(value.to_string()),
            _ => return Err(ConfigError::InvalidConfig(format!("Unknown configuration key: {}", key))),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_config_get_set(key in "(ai_provider\\.|resolution\\.|logging\\.)[a-z_\\.]+", 
                              value in "[a-zA-Z0-9_\\. ]+") {
            let mut config = Config::default();
            
            // Only test valid keys to avoid expected errors
            let valid_key = match key.as_str() {
                "ai_provider.default_provider" | 
                "ai_provider.default_model" | 
                "ai_provider.system_prompt" | 
                "resolution.default_strategy" | 
                "logging.level" | 
                "logging.file" => key.clone(),
                _ => "ai_provider.default_provider".to_string(),
            };
            
            // For timeout, use a valid number
            let valid_value = if valid_key == "ai_provider.timeout_seconds" {
                "30".to_string()
            } else {
                value.clone()
            };
            
            // Set the value
            let result = config.set(&valid_key, &valid_value);
            prop_assert!(result.is_ok());
            
            // Get the value and verify it matches
            let retrieved = config.get(&valid_key);
            prop_assert!(retrieved.is_some());
            prop_assert_eq!(retrieved.unwrap(), valid_value);
        }
    }
    
    #[test]
    fn test_config_load_from_env() {
        // Set environment variables
        env::set_var("GIT_MERGE_AI_PROVIDER", "openai");
        env::set_var("GIT_MERGE_AI_MODEL", "gpt-4");
        env::set_var("GIT_MERGE_LOG_LEVEL", "debug");
        
        let mut config = Config::default();
        config.load_from_env();
        
        assert_eq!(config.ai_provider.default_provider, Some("openai".to_string()));
        assert_eq!(config.ai_provider.default_model, Some("gpt-4".to_string()));
        assert_eq!(config.logging.level, "debug".to_string());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_AI_PROVIDER");
        env::remove_var("GIT_MERGE_AI_MODEL");
        env::remove_var("GIT_MERGE_LOG_LEVEL");
    }
    
    #[test]
    fn test_config_invalid_key() {
        let mut config = Config::default();
        let result = config.set("invalid.key", "value");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_file_extension_strategies() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_EXTENSION_STRATEGY_js", "ai");
        env::set_var("GIT_MERGE_EXTENSION_STRATEGY_md", "simple");
        env::set_var("GIT_MERGE_EXTENSION_STRATEGY_rs", "ai-fallback");
        
        let mut config = Config::default();
        config.load_from_env();
        
        // Check that the extension strategies were loaded correctly
        assert_eq!(config.resolution.extension_strategies.get("js"), Some(&"ai".to_string()));
        assert_eq!(config.resolution.extension_strategies.get("md"), Some(&"simple".to_string()));
        assert_eq!(config.resolution.extension_strategies.get("rs"), Some(&"ai-fallback".to_string()));
        
        // Clean up environment
        env::remove_var("GIT_MERGE_EXTENSION_STRATEGY_js");
        env::remove_var("GIT_MERGE_EXTENSION_STRATEGY_md");
        env::remove_var("GIT_MERGE_EXTENSION_STRATEGY_rs");
    }
    
    #[test]
    fn test_get_strategy_for_file() {
        let mut config = Config::default();
        
        // Set up some extension strategies
        config.resolution.extension_strategies.insert("js".to_string(), "ai".to_string());
        config.resolution.extension_strategies.insert("md".to_string(), "simple".to_string());
        config.resolution.extension_strategies.insert("rs".to_string(), "ai-fallback".to_string());
        
        // Default strategy
        config.resolution.default_strategy = "default-strategy".to_string();
        
        // Test getting strategy for different file extensions
        assert_eq!(config.get_strategy_for_file("file.js"), "ai");
        assert_eq!(config.get_strategy_for_file("file.md"), "simple");
        assert_eq!(config.get_strategy_for_file("file.rs"), "ai-fallback");
        
        // Test getting strategy for a file with no configured extension
        assert_eq!(config.get_strategy_for_file("file.txt"), "default-strategy");
        
        // Test getting strategy for a file with no extension
        assert_eq!(config.get_strategy_for_file("file"), "default-strategy");
    }
}