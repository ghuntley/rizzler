// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::conflict_parser::{ConflictFile, ConflictRegion};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use serde::{Serialize, Deserialize};

/// Error types for AI provider operations
#[derive(Debug, Clone)]
pub enum AIProviderError {
    /// API connection error
    ConnectionError(String),
    
    /// API request error
    RequestError(String),
    
    /// API response parsing error
    ResponseError(String),
    
    /// API authentication error
    AuthError(String),
    
    /// Model not available
    ModelNotAvailable(String),
    
    /// Timeout error
    Timeout(String),
    
    /// Rate limit error
    RateLimit(String),
    
    /// Prompt construction error
    PromptError(String),
    
    /// Missing configuration
    ConfigError(String),
}

impl fmt::Display for AIProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Self::RequestError(msg) => write!(f, "Request error: {}", msg),
            Self::ResponseError(msg) => write!(f, "Response parsing error: {}", msg),
            Self::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            Self::ModelNotAvailable(msg) => write!(f, "Model not available: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::RateLimit(msg) => write!(f, "Rate limit exceeded: {}", msg),
            Self::PromptError(msg) => write!(f, "Prompt construction error: {}", msg),
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl Error for AIProviderError {}

/// AI provider configuration
#[derive(Debug, Clone)]
pub struct AIProviderConfig {
    /// Provider name
    pub name: String,
    
    /// API key
    pub api_key: String,
    
    /// Model name
    pub model: String,
    
    /// Base URL (optional)
    pub base_url: Option<String>,
    
    /// Organization ID (optional)
    pub org_id: Option<String>,
    
    /// Default system prompt
    pub system_prompt: Option<String>,
    
    /// Timeout in seconds
    pub timeout_seconds: u64,
    
    /// Additional provider-specific settings
    pub additional_settings: HashMap<String, String>,
}

/// Response from AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    /// Resolved content
    pub content: String,
    
    /// Explanation of the resolution
    pub explanation: Option<String>,
    
    /// Token usage statistics
    pub token_usage: Option<TokenUsage>,
    
    /// Model used for the response
    pub model: String,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens used
    pub input_tokens: u32,
    
    /// Output tokens used
    pub output_tokens: u32,
    
    /// Total tokens used
    pub total_tokens: u32,
}

/// AI provider trait for conflict resolution
pub trait AIProvider {
    /// Get the name of the provider
    fn name(&self) -> &str;
    
    /// Check if the provider is available (has necessary credentials)
    fn is_available(&self) -> bool;
    
    /// Get the current configuration
    fn config(&self) -> &AIProviderConfig;
    
    /// Resolve a conflict using the AI provider
    fn resolve_conflict(
        &self,
        conflict_file: &ConflictFile,
        conflict: &ConflictRegion,
    ) -> Result<AIResponse, AIProviderError>;
    
    /// Resolve all conflicts in a file
    fn resolve_file(
        &self,
        conflict_file: &ConflictFile,
    ) -> Result<AIResponse, AIProviderError>;
    
    /// Create a system prompt for the AI
    fn create_system_prompt(&self) -> String {
        self.config().system_prompt.clone().unwrap_or_else(|| {
            "You are an expert software developer helping to resolve Git merge conflicts. \
            Analyze the provided code conflicts and resolve them in a way that preserves \
            the intent of both changes whenever possible. When resolving conflicts, consider \
            the context of the entire file and follow the existing code style. Provide a \
            clean resolution without conflict markers.".to_string()
        })
    }
}