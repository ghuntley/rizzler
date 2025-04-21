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
    
    /// Log rotation settings
    #[serde(default)]
    pub rotation: LogRotationConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LogRotationConfig {
    /// Rotation frequency (daily, hourly, never)
    #[serde(default = "default_rotation_frequency")]
    pub frequency: String,
    
    /// Maximum number of log files to keep
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    
    /// Maximum size of each log file (e.g., "10MB")
    #[serde(default = "default_max_file_size")]
    pub max_file_size: String,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            frequency: default_rotation_frequency(),
            max_files: default_max_files(),
            max_file_size: default_max_file_size(),
        }
    }
}

// Default values for configuration
fn default_timeout() -> u64 { 30 }
fn default_strategy() -> String { "ai".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_rotation_frequency() -> String { "daily".to_string() }
fn default_max_files() -> usize { 7 } // Keep logs for a week by default
fn default_max_file_size() -> String { "10MB".to_string() }

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
        
        // First try to load from Git config (repository or global)
        // If this fails, it's not a critical error - we'll just use defaults and env vars
        if let Err(err) = config.load_from_git_config() {
            debug!("Could not load from Git config: {}", err);
        }
        
        // Then load from environment variables (these take precedence over Git config)
        config.load_from_env();
        
        Ok(config)
    }
    
    /// Load the global configuration, returns default config if loading fails
    pub fn load_global() -> Option<Self> {
        Self::load().ok()
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
    
    /// Load configuration from Git config
    pub fn load_from_git_config(&mut self) -> Result<(), ConfigError> {
        // Check if we're in a Git repository by running 'git rev-parse --is-inside-work-tree'
        let git_check = std::process::Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output();
            
        // If the command fails or returns false, we're not in a Git repository
        match git_check {
            Err(e) => {
                return Err(ConfigError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to execute git command: {}", e)
                )));
            },
            Ok(output) => {
                if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim() != "true" {
                    return Err(ConfigError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Not in a Git repository".to_string()
                    )));
                }
            }
        }
        
        // Helper function to run git config for a specific key
        let get_git_config = |key: &str| -> Option<String> {
            let output = std::process::Command::new("git")
                .args(["config", "--get", key])
                .output();
            
            if let Ok(output) = output {
                if output.status.success() {
                    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
            
            None
        };
        
        // Try to get AI provider configuration using Git's simpler key format
        if let Some(value) = get_git_config("merge-ai-resolver.default-provider") {
            self.ai_provider.default_provider = Some(value);
        }
        
        if let Some(value) = get_git_config("merge-ai-resolver.default-model") {
            self.ai_provider.default_model = Some(value);
        }
        
        if let Some(value) = get_git_config("merge-ai-resolver.system-prompt") {
            self.ai_provider.system_prompt = Some(value);
        }
        
        if let Some(value) = get_git_config("merge-ai-resolver.timeout-seconds") {
            if let Ok(timeout) = value.parse::<u64>() {
                self.ai_provider.timeout_seconds = timeout;
            }
        }
        
        // Resolution configuration
        if let Some(value) = get_git_config("merge-ai-resolver.resolution.default_strategy") {
            self.resolution.default_strategy = value;
        }
        
        // Logging configuration
        if let Some(value) = get_git_config("merge-ai-resolver.logging.level") {
            self.logging.level = value;
        }
        
        if let Some(value) = get_git_config("merge-ai-resolver.logging.file") {
            self.logging.file = Some(value);
        }
        
        // For extension strategies, we need to list all keys with the extension_strategy prefix
        let output = std::process::Command::new("git")
            .args(["config", "--get-regexp", r"^merge-ai-resolver.extension_strategy."])
            .output();
        
        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                
                for line in output_str.lines() {
                    if let Some((key, value)) = line.split_once(' ') {
                        if let Some(extension) = key.strip_prefix("merge-ai-resolver.extension_strategy.") {
                            self.resolution.extension_strategies.insert(extension.to_string(), value.trim().to_string());
                            debug!("Added extension strategy mapping from Git config: {} -> {}", extension, value.trim());
                        }
                    }
                }
            }
        }
        
        return Ok(());
    }
    
    /// Load configuration from environment variables
    pub fn load_from_env(&mut self) {
        // AI provider configuration
        if let Ok(provider) = env::var("GIT_MERGE_AI_PROVIDER_DEFAULT") {
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
            _ => {
                // Check if it's an extension strategy
                if key.starts_with("resolution.extension_strategies.") {
                    if let Some(extension) = key.strip_prefix("resolution.extension_strategies.") {
                        self.resolution.extension_strategies.insert(extension.to_string(), value.to_string());
                        return Ok(());
                    }
                }
                
                return Err(ConfigError::InvalidConfig(format!("Unknown configuration key: {}", key)));
            }
        }
        Ok(())
    }
    
    /// Save configuration to Git config
    /// 
    /// This method saves the current configuration to Git config.
    /// If global is true, it saves to global Git config, otherwise to local repository config.
    pub fn save_to_git_config(&self, global: bool) -> Result<(), ConfigError> {
        let config_scope = if global { "--global" } else { "--local" };
        
        // Helper function to run git config command
        let set_git_config = |key: &str, value: &str| -> Result<(), ConfigError> {
            let output = std::process::Command::new("git")
                .args(["config", config_scope, key, value])
                .output()
                .map_err(|e| ConfigError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to execute git config command: {}", e)
                )))?;
            
            if !output.status.success() {
                return Err(ConfigError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Git config command failed: {}", String::from_utf8_lossy(&output.stderr))
                )));
            }
            
            Ok(())
        };
        
        // Save AI provider configuration using Git's dot notation
        if let Some(provider) = &self.ai_provider.default_provider {
            set_git_config("merge-ai-resolver.ai.provider.default_provider", provider)?;
        }
        
        if let Some(model) = &self.ai_provider.default_model {
            set_git_config("merge-ai-resolver.ai.provider.default_model", model)?;
        }
        
        if let Some(prompt) = &self.ai_provider.system_prompt {
            set_git_config("merge-ai-resolver.ai.provider.system_prompt", prompt)?;
        }
        
        set_git_config(
            "merge-ai-resolver.ai.provider.timeout_seconds",
            &self.ai_provider.timeout_seconds.to_string()
        )?;
        
        // Save resolution configuration
        set_git_config(
            "merge-ai-resolver.resolution.default_strategy",
            &self.resolution.default_strategy
        )?;
        
        // Save extension strategies
        for (extension, strategy) in &self.resolution.extension_strategies {
            set_git_config(
                &format!("merge-ai-resolver.extension_strategy.{}", extension),
                strategy
            )?;
        }
        
        // Save logging configuration
        set_git_config("merge-ai-resolver.logging.level", &self.logging.level)?;
        
        if let Some(file) = &self.logging.file {
            set_git_config("merge-ai-resolver.logging.file", file)?;
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
        env::set_var("GIT_MERGE_AI_PROVIDER_DEFAULT", "openai");
        env::set_var("GIT_MERGE_AI_MODEL", "gpt-4");
        env::set_var("GIT_MERGE_LOG_LEVEL", "debug");
        
        let mut config = Config::default();
        config.load_from_env();
        
        assert_eq!(config.ai_provider.default_provider, Some("openai".to_string()));
        assert_eq!(config.ai_provider.default_model, Some("gpt-4".to_string()));
        assert_eq!(config.logging.level, "debug".to_string());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_AI_PROVIDER_DEFAULT");
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