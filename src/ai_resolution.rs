// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderError};
use crate::cache::AIResolutionCache;
use crate::caching_provider::CachingAIProvider;
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use crate::fallback::FallbackResolutionStrategy;
use crate::providers::{OpenAIProvider, ClaudeProvider, GeminiProvider, BedrockProvider};
use crate::resolution_engine::{ResolutionError, ResolutionStrategy};
use crate::retry::{RetryableProvider, RetryConfig};
use std::env;
use std::sync::Arc;
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
        
        // Check if fallback is enabled
        let use_fallback = env::var("GIT_MERGE_AI_USE_FALLBACK")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);
        
        if use_fallback {
            // Get the fallback order, defaulting to all providers
            let fallback_order = env::var("GIT_MERGE_AI_FALLBACK_ORDER")
                .unwrap_or_else(|_| "openai,claude,gemini,bedrock".to_string());
            
            Self::with_fallback(&fallback_order)
        } else {
            Self::with_provider(&provider_name)
        }
    }
    
    /// Create a new AI resolution strategy with a specific provider
    pub fn with_provider(provider_name: &str) -> Result<Self, ResolutionError> {
        // Get retry configuration setting from environment
        let use_retries = env::var("GIT_MERGE_AI_USE_RETRIES")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(true); // Enable retries by default
        
        // Get cache configuration setting from environment
        let use_cache = env::var("GIT_MERGE_AI_USE_CACHE")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(true); // Enable cache by default
            
        let base_provider: Box<dyn AIProvider> = match provider_name.to_lowercase().as_str() {
            "openai" => {
                match OpenAIProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize OpenAI provider: {}", err)
                    )),
                }
            },
            "claude" | "anthropic" => {
                match ClaudeProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize Claude provider: {}", err)
                    )),
                }
            },
            "gemini" | "google" => {
                match GeminiProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize Gemini provider: {}", err)
                    )),
                }
            },
            "bedrock" | "aws" => {
                match BedrockProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize AWS Bedrock provider: {}", err)
                    )),
                }
            },
            _ => return Err(ResolutionError::StrategyError(
                format!("Unknown AI provider: {}", provider_name)
            )),
        };
        
        // First wrap with RetryableProvider if retries are enabled
        let retryable_provider = if use_retries {
            info!("Adding retry capability to {} provider", provider_name);
            Box::new(RetryableProvider::new(base_provider)) as Box<dyn AIProvider>
        } else {
            base_provider
        };
        
        // Then wrap with CachingAIProvider if caching is enabled
        let provider = if use_cache {
            info!("Adding caching capability to {} provider", provider_name);
            Box::new(CachingAIProvider::new(retryable_provider)) as Box<dyn AIProvider>
        } else {
            retryable_provider
        };
        
        Ok(AIResolutionStrategy {
            provider,
        })
    }
    
    /// Create a new AI resolution strategy with fallback between multiple providers
    pub fn with_fallback(providers_list: &str) -> Result<Self, ResolutionError> {
        // Create a fallback resolution strategy
        let fallback_strategy = FallbackResolutionStrategy::with_providers(providers_list)?;
        
        // Get the first provider from the fallback chain
        let provider_names = fallback_strategy.provider_names();
        if provider_names.is_empty() {
            return Err(ResolutionError::StrategyError(
                "No AI providers available for fallback strategy".to_string()
            ));
        }
        
        info!("Created AI resolution strategy with fallback chain: {:?}", provider_names);
        
        // We'll create an AIResolutionStrategy wrapping the fallback strategy
        // by using the first provider from the chain as the primary provider
        Self::with_provider(&provider_names[0])
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
            Err(err) => {
                // Check if we should try to use the fallback strategy
                if let Ok(fallback_enabled) = env::var("GIT_MERGE_AI_USE_FALLBACK") {
                    if fallback_enabled.to_lowercase() == "true" || fallback_enabled == "1" {
                        info!("Primary provider failed: {}", err);
                        info!("Falling back to other providers in the fallback chain");
                        
                        // Get the fallback order from environment
                        let fallback_order = env::var("GIT_MERGE_AI_FALLBACK_ORDER")
                            .unwrap_or_else(|_| "openai,claude,gemini,bedrock".to_string());
                        
                        // Create a fallback strategy
                        match FallbackResolutionStrategy::with_providers(&fallback_order) {
                            Ok(fallback_strategy) => {
                                // Try to resolve with the fallback strategy
                                match fallback_strategy.resolve_conflict(conflict) {
                                    Ok(result) => return Ok(result),
                                    Err(fallback_err) => {
                                        warn!("Fallback strategy also failed: {}", fallback_err);
                                        return Err(fallback_err);
                                    }
                                }
                            },
                            Err(fallback_err) => {
                                error!("Failed to create fallback strategy: {}", fallback_err);
                                // Continue with the original error
                            }
                        }
                    }
                }
                
                // If no fallback available or fallback disabled, return the original error
                Err(map_ai_error_to_resolution_error(err))
            },
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
        
        // Check if fallback is enabled
        let use_fallback = env::var("GIT_MERGE_AI_USE_FALLBACK")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);
        
        if use_fallback {
            // Get the fallback order, defaulting to all providers
            let fallback_order = env::var("GIT_MERGE_AI_FALLBACK_ORDER")
                .unwrap_or_else(|_| "openai,claude,gemini,bedrock".to_string());
            
            Self::with_fallback(&fallback_order)
        } else {
            Self::with_provider(&provider_name)
        }
    }
    
    /// Create a new AI file resolution strategy with a specific provider
    pub fn with_provider(provider_name: &str) -> Result<Self, ResolutionError> {
        // Get retry configuration setting from environment
        let use_retries = env::var("GIT_MERGE_AI_USE_RETRIES")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(true); // Enable retries by default
        
        // Get cache configuration setting from environment
        let use_cache = env::var("GIT_MERGE_AI_USE_CACHE")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(true); // Enable cache by default
            
        let base_provider: Box<dyn AIProvider> = match provider_name.to_lowercase().as_str() {
            "openai" => {
                match OpenAIProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize OpenAI provider: {}", err)
                    )),
                }
            },
            "claude" | "anthropic" => {
                match ClaudeProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize Claude provider: {}", err)
                    )),
                }
            },
            "gemini" | "google" => {
                match GeminiProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize Gemini provider: {}", err)
                    )),
                }
            },
            "bedrock" | "aws" => {
                match BedrockProvider::new() {
                    Ok(provider) => Box::new(provider),
                    Err(err) => return Err(ResolutionError::StrategyError(
                        format!("Failed to initialize AWS Bedrock provider: {}", err)
                    )),
                }
            },
            _ => return Err(ResolutionError::StrategyError(
                format!("Unknown AI provider: {}", provider_name)
            )),
        };
        
        // First wrap with RetryableProvider if retries are enabled
        let retryable_provider = if use_retries {
            info!("Adding retry capability to {} provider for file resolution", provider_name);
            Box::new(RetryableProvider::new(base_provider)) as Box<dyn AIProvider>
        } else {
            base_provider
        };
        
        // Then wrap with CachingAIProvider if caching is enabled
        let provider = if use_cache {
            info!("Adding caching capability to {} provider for file resolution", provider_name);
            Box::new(CachingAIProvider::new(retryable_provider)) as Box<dyn AIProvider>
        } else {
            retryable_provider
        };
        
        Ok(AIFileResolutionStrategy {
            provider,
        })
    }
    
    /// Create a new AI file resolution strategy with fallback between multiple providers
    pub fn with_fallback(providers_list: &str) -> Result<Self, ResolutionError> {
        // Create a fallback resolution strategy
        let fallback_strategy = FallbackResolutionStrategy::with_providers(providers_list)?;
        
        // Get the first provider from the fallback chain
        let provider_names = fallback_strategy.provider_names();
        if provider_names.is_empty() {
            return Err(ResolutionError::StrategyError(
                "No AI providers available for fallback strategy".to_string()
            ));
        }
        
        info!("Created AI file resolution strategy with fallback chain: {:?}", provider_names);
        
        // Similar to AIResolutionStrategy, we'll use the first provider from the chain
        // as the primary provider. The fallback will be handled at a higher level.
        Self::with_provider(&provider_names[0])
    }
    
    /// Resolve all conflicts in a file at once
    pub fn resolve_file(&self, conflict_file: &ConflictFile) -> Result<String, ResolutionError> {
        match self.provider.resolve_file(conflict_file) {
            Ok(response) => Ok(response.content),
            Err(err) => {
                // Check if we should try to use the fallback strategy
                if let Ok(fallback_enabled) = env::var("GIT_MERGE_AI_USE_FALLBACK") {
                    if fallback_enabled.to_lowercase() == "true" || fallback_enabled == "1" {
                        info!("Primary provider failed: {}", err);
                        info!("Falling back to other providers in the fallback chain");
                        
                        // Get the fallback order from environment
                        let fallback_order = env::var("GIT_MERGE_AI_FALLBACK_ORDER")
                            .unwrap_or_else(|_| "openai,claude,gemini,bedrock".to_string());
                        
                        // Create a fallback strategy
                        match FallbackResolutionStrategy::with_providers(&fallback_order) {
                            Ok(fallback_strategy) => {
                                // Try to resolve with the fallback strategy
                                match fallback_strategy.resolve_file(conflict_file) {
                                    Ok(result) => return Ok(result),
                                    Err(fallback_err) => {
                                        warn!("Fallback strategy also failed: {}", fallback_err);
                                        return Err(fallback_err);
                                    }
                                }
                            },
                            Err(fallback_err) => {
                                error!("Failed to create fallback strategy: {}", fallback_err);
                                // Continue with the original error
                            }
                        }
                    }
                }
                
                // If no fallback available or fallback disabled, return the original error
                Err(map_ai_error_to_resolution_error(err))
            },
        }
    }
}

/// Map AI provider errors to resolution errors
pub fn map_ai_error_to_resolution_error(err: AIProviderError) -> ResolutionError {
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
    
    // Helper function to set up environment for retry testing
    fn setup_retry_test() {
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        env::set_var("GIT_MERGE_AI_USE_RETRIES", "true");
        env::set_var("GIT_MERGE_AI_MAX_RETRIES", "2"); // Use a smaller value for faster tests
        env::set_var("GIT_MERGE_AI_INITIAL_BACKOFF_MS", "1"); // Use a small value for faster tests
    }
    
    // Helper function to clean up environment after retry testing
    fn cleanup_retry_test() {
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
        env::remove_var("GIT_MERGE_AI_USE_RETRIES");
        env::remove_var("GIT_MERGE_AI_MAX_RETRIES");
        env::remove_var("GIT_MERGE_AI_INITIAL_BACKOFF_MS");
    }
    
    #[test]
    fn test_ai_resolution_strategy_initialization_openai() {
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
    fn test_ai_resolution_strategy_initialization_claude() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_CLAUDE_API_KEY", "test-api-key");
        env::set_var("GIT_MERGE_AI_PROVIDER", "claude");
        
        // Test initialization with default provider (now claude)
        let strategy = AIResolutionStrategy::new();
        assert!(strategy.is_ok());
        
        // Test initialization with specific provider
        let strategy = AIResolutionStrategy::with_provider("claude");
        assert!(strategy.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_CLAUDE_API_KEY");
        env::remove_var("GIT_MERGE_AI_PROVIDER");
    }
    
    #[test]
    fn test_ai_resolution_strategy_initialization_gemini() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
        env::set_var("GIT_MERGE_AI_PROVIDER", "gemini");
        
        // Test initialization with default provider (now gemini)
        let strategy = AIResolutionStrategy::new();
        assert!(strategy.is_ok());
        
        // Test initialization with specific provider
        let strategy = AIResolutionStrategy::with_provider("gemini");
        assert!(strategy.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_GEMINI_API_KEY");
        env::remove_var("GIT_MERGE_AI_PROVIDER");
    }
    
    #[test]
    fn test_ai_resolution_strategy_conflict_handling_openai() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy
        let strategy = AIResolutionStrategy::with_provider("openai").unwrap();
        
        // Check if it can handle conflicts
        assert!(strategy.can_handle(&conflict));
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_ai_resolution_strategy_conflict_handling_claude() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_CLAUDE_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy
        let strategy = AIResolutionStrategy::with_provider("claude").unwrap();
        
        // Check if it can handle conflicts
        assert!(strategy.can_handle(&conflict));
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_CLAUDE_API_KEY");
    }
    
    #[test]
    fn test_ai_resolution_strategy_conflict_handling_gemini() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy
        let strategy = AIResolutionStrategy::with_provider("gemini").unwrap();
        
        // Check if it can handle conflicts
        assert!(strategy.can_handle(&conflict));
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_GEMINI_API_KEY");
    }
    
    #[test]
    fn test_ai_file_resolution_strategy_openai() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = create_test_conflict_file(vec![conflict]);
        
        // Create strategy
        let strategy = AIFileResolutionStrategy::with_provider("openai").unwrap();
        
        // Test resolving a file
        let result = strategy.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_ai_file_resolution_strategy_claude() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_CLAUDE_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = create_test_conflict_file(vec![conflict]);
        
        // Create strategy
        let strategy = AIFileResolutionStrategy::with_provider("claude").unwrap();
        
        // Test resolving a file
        let result = strategy.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_CLAUDE_API_KEY");
    }
    
    #[test]
    fn test_ai_file_resolution_strategy_gemini() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = create_test_conflict_file(vec![conflict]);
        
        // Create strategy
        let strategy = AIFileResolutionStrategy::with_provider("gemini").unwrap();
        
        // Test resolving a file
        let result = strategy.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_GEMINI_API_KEY");
    }
    
    #[test]
    fn test_ai_resolution_strategy_initialization_bedrock() {
        // Set environment variables for testing
        env::set_var("AWS_ACCESS_KEY_ID", "test-access-key");
        env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret-key");
        env::set_var("AWS_REGION", "us-east-1");
        env::set_var("GIT_MERGE_AI_PROVIDER", "bedrock");
        
        // Test initialization with default provider (now bedrock)
        let strategy = AIResolutionStrategy::new();
        assert!(strategy.is_ok());
        
        // Test initialization with specific provider
        let strategy = AIResolutionStrategy::with_provider("bedrock");
        assert!(strategy.is_ok());
        
        // Clean up environment
        env::remove_var("AWS_ACCESS_KEY_ID");
        env::remove_var("AWS_SECRET_ACCESS_KEY");
        env::remove_var("AWS_REGION");
        env::remove_var("GIT_MERGE_AI_PROVIDER");
    }
    
    #[test]
    fn test_ai_resolution_strategy_conflict_handling_bedrock() {
        // Set environment variables for testing
        env::set_var("AWS_ACCESS_KEY_ID", "test-access-key");
        env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret-key");
        env::set_var("AWS_REGION", "us-east-1");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy
        let strategy = AIResolutionStrategy::with_provider("bedrock").unwrap();
        
        // Check if it can handle conflicts
        assert!(strategy.can_handle(&conflict));
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("AWS_ACCESS_KEY_ID");
        env::remove_var("AWS_SECRET_ACCESS_KEY");
        env::remove_var("AWS_REGION");
    }
    
    #[test]
    fn test_ai_file_resolution_strategy_bedrock() {
        // Set environment variables for testing
        env::set_var("AWS_ACCESS_KEY_ID", "test-access-key");
        env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret-key");
        env::set_var("AWS_REGION", "us-east-1");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = create_test_conflict_file(vec![conflict]);
        
        // Create strategy
        let strategy = AIFileResolutionStrategy::with_provider("bedrock").unwrap();
        
        // Test resolving a file
        let result = strategy.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("AWS_ACCESS_KEY_ID");
        env::remove_var("AWS_SECRET_ACCESS_KEY");
        env::remove_var("AWS_REGION");
    }
    
    #[test]
    fn test_ai_resolution_strategy_with_retries() {
        // Set up environment for retry testing
        setup_retry_test();
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy with retries enabled
        let strategy = AIResolutionStrategy::with_provider("openai").unwrap();
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        cleanup_retry_test();
    }
    
    #[test]
    fn test_ai_file_resolution_strategy_with_retries() {
        // Set up environment for retry testing
        setup_retry_test();
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = create_test_conflict_file(vec![conflict]);
        
        // Create strategy with retries enabled
        let strategy = AIFileResolutionStrategy::with_provider("openai").unwrap();
        
        // Test resolving a file
        let result = strategy.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Clean up environment
        cleanup_retry_test();
    }
    
    #[test]
    fn test_retry_disabled() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
        env::set_var("GIT_MERGE_AI_USE_RETRIES", "false"); // Explicitly disable retries
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy with retries disabled
        let strategy = AIResolutionStrategy::with_provider("openai").unwrap();
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("GIT_MERGE_OPENAI_API_KEY");
        env::remove_var("GIT_MERGE_AI_USE_RETRIES");
    }
}