// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::conflict_parser::{ConflictFile, ConflictRegion};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use tracing::{debug, info, warn, error};

/// Error types for AI provider operations
#[derive(Debug)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// OpenAI provider implementation
pub struct OpenAIProvider {
    config: AIProviderConfig,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new() -> Result<Self, AIProviderError> {
        // Get API key from environment variable
        let api_key = env::var("GIT_MERGE_OPENAI_API_KEY")
            .map_err(|_| AIProviderError::ConfigError(
                "Missing OpenAI API key. Set GIT_MERGE_OPENAI_API_KEY environment variable".to_string()
            ))?;
        
        // Get other configuration from environment variables
        let base_url = env::var("GIT_MERGE_OPENAI_BASE_URL").ok();
        let org_id = env::var("GIT_MERGE_OPENAI_ORG_ID").ok();
        let model = env::var("GIT_MERGE_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4-turbo".to_string());
        let timeout_seconds = env::var("GIT_MERGE_AI_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        
        // Get custom system prompt if provided
        let system_prompt = env::var("GIT_MERGE_AI_SYSTEM_PROMPT").ok();
        
        // Create additional settings map
        let mut additional_settings = HashMap::new();
        if let Ok(max_tokens) = env::var("GIT_MERGE_OPENAI_MAX_TOKENS") {
            additional_settings.insert("max_tokens".to_string(), max_tokens);
        }
        
        Ok(OpenAIProvider {
            config: AIProviderConfig {
                name: "openai".to_string(),
                api_key,
                model,
                base_url,
                org_id,
                system_prompt,
                timeout_seconds,
                additional_settings,
            },
        })
    }
    
    /// Create a user prompt for resolving a specific conflict
    fn create_user_prompt(&self, conflict_file: &ConflictFile, conflict: &ConflictRegion) -> String {
        format!(
            "I need help resolving a Git merge conflict in the file: {}\n\n\
            The file contains a conflict between line {} and {}:\n\n\
            OUR VERSION (current branch):\n```\n{}```\n\n\
            THEIR VERSION (incoming branch):\n```\n{}```\n\n\
            Please resolve this conflict and provide only the final resolved content that should replace \
            the conflict. Preserve the intent of both changes if possible or choose the most appropriate \
            version if they are in direct conflict. Do not include conflict markers in your response.",
            conflict_file.path,
            conflict.start_line,
            conflict.end_line,
            conflict.our_content,
            conflict.their_content
        )
    }
    
    /// Create a user prompt for resolving an entire file
    fn create_file_prompt(&self, conflict_file: &ConflictFile) -> String {
        let mut conflicts_text = String::new();
        
        for (i, conflict) in conflict_file.conflicts.iter().enumerate() {
            conflicts_text.push_str(&format!(
                "CONFLICT {}:\nBetween lines {} and {}\n\
                OUR VERSION:\n```\n{}```\n\
                THEIR VERSION:\n```\n{}```\n\n",
                i + 1,
                conflict.start_line,
                conflict.end_line,
                conflict.our_content,
                conflict.their_content
            ));
        }
        
        format!(
            "I need help resolving Git merge conflicts in the file: {}\n\n\
            The file has {} conflict(s):\n\n{}\n\
            Please provide the entire resolved file content with all conflicts resolved. \
            Preserve the intent of both changes whenever possible. \
            Do not include conflict markers in your response.",
            conflict_file.path,
            conflict_file.conflicts.len(),
            conflicts_text
        )
    }
    
    /// Parse the response from OpenAI
    fn parse_response(&self, response_text: &str) -> Result<String, AIProviderError> {
        // For now, a simple implementation that just returns the text
        // In a real implementation, we would need to handle code blocks and formatting
        Ok(response_text.to_string())
    }
}

impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }
    
    fn is_available(&self) -> bool {
        !self.config.api_key.is_empty()
    }
    
    fn config(&self) -> &AIProviderConfig {
        &self.config
    }
    
    fn resolve_conflict(
        &self,
        conflict_file: &ConflictFile,
        conflict: &ConflictRegion,
    ) -> Result<AIResponse, AIProviderError> {
        // Check if the provider is available
        if !self.is_available() {
            return Err(AIProviderError::ConfigError(
                "OpenAI provider is not available. API key is missing".to_string()
            ));
        }
        
        let system_prompt = self.create_system_prompt();
        let user_prompt = self.create_user_prompt(conflict_file, conflict);
        
        info!("Sending conflict to OpenAI for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // This is a placeholder - in a real implementation, we would send the request to the OpenAI API
        // and parse the response. For now, we'll just return a mock response for testing.
        
        // Mock response - this would be replaced with actual API call logic
        let mock_response = "This is a mock response from OpenAI.\nIn a real implementation, we would call the OpenAI API and get a real response.";
        
        let resolved_content = self.parse_response(mock_response)?;
        
        Ok(AIResponse {
            content: resolved_content,
            explanation: Some("Mock explanation for testing".to_string()),
            token_usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
            }),
            model: self.config.model.clone(),
        })
    }
    
    fn resolve_file(
        &self,
        conflict_file: &ConflictFile,
    ) -> Result<AIResponse, AIProviderError> {
        // Check if the provider is available
        if !self.is_available() {
            return Err(AIProviderError::ConfigError(
                "OpenAI provider is not available. API key is missing".to_string()
            ));
        }
        
        let system_prompt = self.create_system_prompt();
        let user_prompt = self.create_file_prompt(conflict_file);
        
        info!("Sending entire file to OpenAI for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // This is a placeholder - in a real implementation, we would send the request to the OpenAI API
        // and parse the response. For now, we'll just return a mock response for testing.
        
        // Mock response - this would be replaced with actual API call logic
        let mock_response = "This is a mock response from OpenAI for the entire file.\nIn a real implementation, we would call the OpenAI API and get a real response.";
        
        let resolved_content = self.parse_response(mock_response)?;
        
        Ok(AIResponse {
            content: resolved_content,
            explanation: Some("Mock explanation for testing".to_string()),
            token_usage: Some(TokenUsage {
                input_tokens: 200,
                output_tokens: 100,
                total_tokens: 300,
            }),
            model: self.config.model.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict_parser::ConflictRegion;
    use std::env;
    use proptest::prelude::*;
    
    // Helper function to create a test conflict region
    fn create_test_conflict(our_content: &str, their_content: &str) -> ConflictRegion {
        ConflictRegion {
            base_content: String::new(),
            our_content: our_content.to_string(),
            their_content: their_content.to_string(),
            start_line: 1,
            end_line: 5,
        }
    }
    
    // Helper function to create a test conflict file
    fn create_test_conflict_file(conflicts: Vec<ConflictRegion>) -> ConflictFile {
        ConflictFile {
            path: "test.txt".to_string(),
            conflicts,
            content: "<<<<<<< HEAD\nTest content\n=======\nTheir content\n>>>>>>> branch-name\n".to_string(),
        }
    }
    
    #[test]
    fn test_openai_provider_config() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        env::set_var("GIT_MERGE_OPENAI_MODEL", "gpt-4");
        env::set_var("GIT_MERGE_OPENAI_BASE_URL", "https://test-api.openai.com");
        env::set_var("GIT_MERGE_OPENAI_ORG_ID", "test-org");
        env::set_var("GIT_MERGE_AI_SYSTEM_PROMPT", "Test system prompt");
        env::set_var("GIT_MERGE_AI_TIMEOUT", "30");
        
        // Create provider
        let provider = OpenAIProvider::new().unwrap();
        
        // Check configuration
        assert_eq!(provider.config().api_key, "test-api-key");
        assert_eq!(provider.config().model, "gpt-4");
        assert_eq!(provider.config().base_url, Some("https://test-api.openai.com".to_string()));
        assert_eq!(provider.config().org_id, Some("test-org".to_string()));
        assert_eq!(provider.config().system_prompt, Some("Test system prompt".to_string()));
        assert_eq!(provider.config().timeout_seconds, 30);
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
        env::remove_var("GIT_MERGE_OPENAI_MODEL");
        env::remove_var("GIT_MERGE_OPENAI_BASE_URL");
        env::remove_var("GIT_MERGE_OPENAI_ORG_ID");
        env::remove_var("GIT_MERGE_AI_SYSTEM_PROMPT");
        env::remove_var("GIT_MERGE_AI_TIMEOUT");
    }
    
    #[test]
    fn test_create_system_prompt() {
        // Set the API key for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Test default system prompt
        {
            let provider = OpenAIProvider::new().unwrap();
            let system_prompt = provider.create_system_prompt();
            assert!(system_prompt.contains("Git merge conflicts"));
        }
        
        // Test custom system prompt
        {
            env::set_var("GIT_MERGE_AI_SYSTEM_PROMPT", "Custom system prompt");
            let provider = OpenAIProvider::new().unwrap();
            let system_prompt = provider.create_system_prompt();
            assert_eq!(system_prompt, "Custom system prompt");
            env::remove_var("GIT_MERGE_AI_SYSTEM_PROMPT");
        }
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_create_user_prompt() {
        // Set the API key for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = OpenAIProvider::new().unwrap();
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\nwith multiple lines\n", "Their content\nalso with lines\n");
        let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Create user prompt
        let prompt = provider.create_user_prompt(&conflict_file, &conflict);
        
        // Check prompt content
        assert!(prompt.contains("Git merge conflict"));
        assert!(prompt.contains("OUR VERSION"));
        assert!(prompt.contains("THEIR VERSION"));
        assert!(prompt.contains("Our content"));
        assert!(prompt.contains("Their content"));
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_resolve_conflict() {
        // Set the API key for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = OpenAIProvider::new().unwrap();
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Resolve conflict
        let result = provider.resolve_conflict(&conflict_file, &conflict);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.content.is_empty());
        assert!(response.explanation.is_some());
        assert!(response.token_usage.is_some());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
    
    proptest! {
        #[test]
        fn test_create_user_prompt_prop(our_content in "[\w\s]{1,100}", their_content in "[\w\s]{1,100}") {
            // Set the API key for testing
            env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
            
            // Create a provider
            let provider = OpenAIProvider::new().unwrap();
            
            // Create a test conflict
            let conflict = create_test_conflict(&our_content, &their_content);
            let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
            
            // Create user prompt
            let prompt = provider.create_user_prompt(&conflict_file, &conflict);
            
            // Check that the prompt contains the content we provided
            prop_assert!(prompt.contains(&our_content));
            prop_assert!(prompt.contains(&their_content));
            
            // Clean up environment
            env::remove_var("GIT_MERGE_OPENAI_API_KEY");
        }
    }
}