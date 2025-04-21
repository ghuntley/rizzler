// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderError, OpenAIProvider};
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use crate::resolution_engine::{ResolutionError, ResolutionStrategy};
use std::env;
use tracing::{debug, info, warn, error};

/// AI-based resolution strategy using supported AI providers
pub struct AIResolutionStrategy {
    provider: Box<dyn AIProvider>,
}

impl AIResolutionStrategy {
    /// Create a new AI resolution strategy with the default provider
    pub fn new() -> Result<Self, ResolutionError> {
        // Use the environment variable to determine the provider, defaulting to OpenAI
        let provider_name = env::var("GIT_MERGE_AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        
        Self::with_provider(&provider_name)
    }
    
    /// Create a new AI resolution strategy with a specific provider
    pub fn with_provider(provider_name: &str) -> Result<Self, ResolutionError> {
        let provider: Box<dyn AIProvider> = match provider_name.to_lowercase().as_str() {
            "openai" => {
                match OpenAIProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize OpenAI provider: {}", err)
                    )),
                }
            },
            // TODO: Add more providers as they are implemented
            _ => return Err(ResolutionError::StrategyError(
                format!("Unknown AI provider: {}", provider_name)
            )),
        };
        
        Ok(AIResolutionStrategy {
            provider,
        })
    }
}

impl ResolutionStrategy for AIResolutionStrategy {
    fn name(&self) -> &str {
        "ai"
    }
    
    fn can_handle(&self, _conflict: &ConflictRegion) -> bool {
        // AI can theoretically handle any conflict if the provider is available
        self.provider.is_available()
    }
    
    fn resolve_conflict(&self, conflict: &ConflictRegion) -> Result<String, ResolutionError> {
        // We need the whole file for context, but we don't have it in this method signature.
        // In a real implementation, we might need to adapt the ResolutionStrategy trait or
        // store the ConflictFile in the AIResolutionStrategy.
        // For now, we'll create a minimal ConflictFile with just this conflict.
        
        let conflict_file = ConflictFile {
            path: "file.txt".to_string(), // Placeholder path
            conflicts: vec![conflict.clone()],
            content: format!(
                "<<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name",
                conflict.our_content,
                conflict.their_content
            ),
        };
        
        match self.provider.resolve_conflict(&conflict_file, conflict) {
            Ok(response) => Ok(response.content),
            Err(err) => Err(map_ai_error_to_resolution_error(err)),
        }
    }
}

/// AI resolution strategy that resolves all conflicts in a file at once
pub struct AIFileResolutionStrategy {
    provider: Box<dyn AIProvider>,
}

impl AIFileResolutionStrategy {
    /// Create a new AI file resolution strategy with the default provider
    pub fn new() -> Result<Self, ResolutionError> {
        // Use the environment variable to determine the provider, defaulting to OpenAI
        let provider_name = env::var("GIT_MERGE_AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        
        Self::with_provider(&provider_name)
    }
    
    /// Create a new AI file resolution strategy with a specific provider
    pub fn with_provider(provider_name: &str) -> Result<Self, ResolutionError> {
        let provider: Box<dyn AIProvider> = match provider_name.to_lowercase().as_str() {
            "openai" => {
                match OpenAIProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize OpenAI provider: {}", err)
                    )),
                }
            },
            // TODO: Add more providers as they are implemented
            _ => return Err(ResolutionError::StrategyError(
                format!("Unknown AI provider: {}", provider_name)
            )),
        };
        
        Ok(AIFileResolutionStrategy {
            provider,
        })
    }
    
    /// Resolve all conflicts in a file at once
    pub fn resolve_file(&self, conflict_file: &ConflictFile) -> Result<String, ResolutionError> {
        match self.provider.resolve_file(conflict_file) {
            Ok(response) => Ok(response.content),
            Err(err) => Err(map_ai_error_to_resolution_error(err)),
        }
    }
}

/// Map AI provider errors to resolution errors
fn map_ai_error_to_resolution_error(err: AIProviderError) -> ResolutionError {
    match err {
        AIProviderError::ConnectionError(msg) |
        AIProviderError::RequestError(msg) |
        AIProviderError::ResponseError(msg) |
        AIProviderError::AuthError(msg) |
        AIProviderError::ModelNotAvailable(msg) |
        AIProviderError::Timeout(msg) |
        AIProviderError::RateLimit(msg) |
        AIProviderError::PromptError(msg) |
        AIProviderError::ConfigError(msg) => {
            ResolutionError::StrategyError(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
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
    fn test_ai_resolution_strategy_initialization() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Test initialization with default provider
        let strategy = AIResolutionStrategy::new();
        assert!(strategy.is_ok());
        
        // Test initialization with specific provider
        let strategy = AIResolutionStrategy::with_provider("openai");
        assert!(strategy.is_ok());
        
        // Test initialization with unknown provider
        let strategy = AIResolutionStrategy::with_provider("unknown");
        assert!(strategy.is_err());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_ai_resolution_strategy_conflict_handling() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy
        let strategy = AIResolutionStrategy::new().unwrap();
        
        // Check if it can handle conflicts
        assert!(strategy.can_handle(&conflict));
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_ai_file_resolution_strategy() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = create_test_conflict_file(vec![conflict]);
        
        // Create strategy
        let strategy = AIFileResolutionStrategy::new().unwrap();
        
        // Test resolving a file
        let result = strategy.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
}