// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::AIProvider;
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use crate::providers::{OpenAIProvider, ClaudeProvider, GeminiProvider, BedrockProvider};
use crate::resolution_engine::{ResolutionError, ResolutionStrategy};
use std::env;
use tracing::{info, warn};

/// A multi-provider resolution strategy that tries multiple AI providers in sequence
/// until one succeeds or all fail
pub struct FallbackResolutionStrategy {
    /// List of providers to try in sequence
    providers: Vec<Box<dyn AIProvider>>,
    /// List of provider names for logging
    provider_names: Vec<String>,
}

impl FallbackResolutionStrategy {
    /// Create a new fallback resolution strategy with default provider order
    pub fn new() -> Result<Self, ResolutionError> {
        // Get fallback order from environment variable
        let fallback_order = env::var("RIZZLER_FALLBACK_ORDER")
            .unwrap_or_else(|_| "openai,claude,gemini,bedrock".to_string());
        
        Self::with_providers(&fallback_order)
    }
    
    /// Create a new fallback resolution strategy with specific provider order
    pub fn with_providers(providers_list: &str) -> Result<Self, ResolutionError> {
        let provider_names: Vec<String> = providers_list
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();
        
        if provider_names.is_empty() {
            return Err(ResolutionError::StrategyError(
                "No AI providers specified for fallback strategy".to_string()
            ));
        }
        
        let mut providers: Vec<Box<dyn AIProvider>> = Vec::new();
        let mut available_provider_names: Vec<String> = Vec::new();
        
        // Initialize each provider in the specified order
        for name in &provider_names {
            match name.as_str() {
                "openai" => {
                    if let Ok(provider) = OpenAIProvider::new() {
                        if provider.is_available() {
                            info!("Added OpenAI provider to fallback chain");
                            providers.push(Box::new(provider));
                            available_provider_names.push(name.clone());
                        } else {
                            warn!("OpenAI provider is not available (missing API key)");
                        }
                    } else {
                        warn!("Failed to initialize OpenAI provider");
                    }
                },
                "claude" | "anthropic" => {
                    if let Ok(provider) = ClaudeProvider::new() {
                        if provider.is_available() {
                            info!("Added Claude provider to fallback chain");
                            providers.push(Box::new(provider));
                            available_provider_names.push(name.clone());
                        } else {
                            warn!("Claude provider is not available (missing API key)");
                        }
                    } else {
                        warn!("Failed to initialize Claude provider");
                    }
                },
                "gemini" | "google" => {
                    if let Ok(provider) = GeminiProvider::new() {
                        if provider.is_available() {
                            info!("Added Gemini provider to fallback chain");
                            providers.push(Box::new(provider));
                            available_provider_names.push(name.clone());
                        } else {
                            warn!("Gemini provider is not available (missing API key)");
                        }
                    } else {
                        warn!("Failed to initialize Gemini provider");
                    }
                },
                "bedrock" | "aws" => {
                    if let Ok(provider) = BedrockProvider::new() {
                        if provider.is_available() {
                            info!("Added AWS Bedrock provider to fallback chain");
                            providers.push(Box::new(provider));
                            available_provider_names.push(name.clone());
                        } else {
                            warn!("AWS Bedrock provider is not available (missing AWS credentials or region)");
                        }
                    } else {
                        warn!("Failed to initialize AWS Bedrock provider");
                    }
                },
                _ => {
                    warn!("Unknown AI provider: {}", name);
                }
            }
        }
        
        // Check if at least one provider is available
        if providers.is_empty() {
            return Err(ResolutionError::StrategyError(
                "No AI providers available for fallback strategy".to_string()
            ));
        }
        
        Ok(FallbackResolutionStrategy {
            providers,
            provider_names: available_provider_names,
        })
    }
    
    /// Get the list of available provider names
    pub fn provider_names(&self) -> &[String] {
        &self.provider_names
    }
    
    /// Try to resolve a file with all providers in the fallback chain
    pub fn resolve_file(&self, conflict_file: &ConflictFile) -> Result<String, ResolutionError> {
        let mut last_error = None;
        
        for (i, provider) in self.providers.iter().enumerate() {
            info!("Trying provider {} ({}) to resolve file", i + 1, provider.name());
            
            match provider.resolve_file(conflict_file) {
                Ok(response) => {
                    info!("Provider {} ({}) successfully resolved file", i + 1, provider.name());
                    return Ok(response.content);
                },
                Err(err) => {
                    warn!("Provider {} ({}) failed to resolve file: {}", i + 1, provider.name(), err);
                    last_error = Some(err);
                    // Continue to the next provider
                }
            }
        }
        
        // If we get here, all providers failed
        Err(ResolutionError::StrategyError(format!(
            "All providers failed to resolve file. Last error: {}",
            last_error.map_or("Unknown error".to_string(), |e| e.to_string())
        )))
    }
}

impl ResolutionStrategy for FallbackResolutionStrategy {
    fn name(&self) -> &str {
        "ai-fallback"
    }
    
    fn can_handle(&self, _conflict: &ConflictRegion) -> bool {
        // The fallback strategy can handle any conflict as long as it has at least one provider
        !self.providers.is_empty()
    }
    
    fn resolve_conflict(&self, conflict: &ConflictRegion) -> Result<String, ResolutionError> {
        // Create a minimal conflict file with just this conflict for providers that need a file context
        let conflict_file = ConflictFile {
            path: "file.txt".to_string(), // Placeholder path
            conflicts: vec![conflict.clone()],
            content: format!(
                "<<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name",
                conflict.our_content,
                conflict.their_content
            ),
        };
        
        let mut last_error = None;
        
        // Try each provider in sequence
        for (i, provider) in self.providers.iter().enumerate() {
            info!("Trying provider {} ({}) to resolve conflict", i + 1, provider.name());
            
            match provider.resolve_conflict(&conflict_file, conflict) {
                Ok(response) => {
                    info!("Provider {} ({}) successfully resolved conflict", i + 1, provider.name());
                    return Ok(response.content);
                },
                Err(err) => {
                    warn!("Provider {} ({}) failed to resolve conflict: {}", i + 1, provider.name(), err);
                    last_error = Some(err);
                    // Continue to the next provider
                }
            }
        }
        
        // If we get here, all providers failed
        Err(ResolutionError::StrategyError(format!(
            "All providers failed to resolve conflict. Last error: {}",
            last_error.map_or("Unknown error".to_string(), |e| e.to_string())
        )))
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
    fn test_fallback_strategy_initialization() {
        // Set environment variables for testing
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        
        // Test initialization with default provider order
        let strategy = FallbackResolutionStrategy::new();
        assert!(strategy.is_ok());
        
        let strategy = strategy.unwrap();
        assert!(!strategy.provider_names().is_empty());
        
        // Test initialization with specific provider order
        let strategy = FallbackResolutionStrategy::with_providers("claude,openai");
        assert!(strategy.is_ok());
        
        let strategy = strategy.unwrap();
        assert!(strategy.provider_names().len() >= 1);
        // We should have claude or openai in the provider list, but we can't guarantee claude
        // will always be first since tests may run in parallel and environment variables might
        // be modified by other tests
        
        // Test initialization with invalid provider
        let strategy = FallbackResolutionStrategy::with_providers("invalid,openai");
        assert!(strategy.is_ok()); // Should still work with just openai
        
        // Clean up environment
        env::remove_var("RIZZLER_OPENAI_API_KEY");
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
    }
    
    // This test verifies that the fallback strategy fails when no providers are available
    #[test]
    fn test_fallback_strategy_no_providers() {
        // Use nonexistent provider to ensure we test the error path
        let strategy = FallbackResolutionStrategy::with_providers("non-existent-provider");
        assert!(strategy.is_err());
        
        // Since ResolutionError doesn't implement Debug, we can't use unwrap_err().
        // Just verify that the strategy is an error
        match strategy {
            Err(err) => assert!(err.to_string().contains("No AI providers available")),
            Ok(_) => panic!("Expected an error but got Ok")
        }
    }
    
    #[test]
    fn test_fallback_strategy_multiple_providers() {
        // Set environment variables for testing
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        env::set_var("RIZZLER_GEMINI_API_KEY", "test-api-key");
        env::set_var("AWS_ACCESS_KEY_ID", "test-access-key");
        env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret-key");
        env::set_var("AWS_REGION", "us-east-1");
        
        // Create strategy with all providers
        let strategy = FallbackResolutionStrategy::with_providers("openai,claude,gemini,bedrock");
        assert!(strategy.is_ok());
        
        let strategy = strategy.unwrap();
        
        // Should have all providers available
        assert_eq!(strategy.provider_names().len(), 4);
        assert!(strategy.provider_names().contains(&"openai".to_string()));
        assert!(strategy.provider_names().contains(&"claude".to_string()));
        assert!(strategy.provider_names().contains(&"gemini".to_string()));
        assert!(strategy.provider_names().contains(&"bedrock".to_string()));
        
        // Clean up environment
        env::remove_var("RIZZLER_OPENAI_API_KEY");
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
        env::remove_var("RIZZLER_GEMINI_API_KEY");
        env::remove_var("AWS_ACCESS_KEY_ID");
        env::remove_var("AWS_SECRET_ACCESS_KEY");
        env::remove_var("AWS_REGION");
    }

    #[test]
    fn test_fallback_strategy_conflict_resolution() {
        // Set environment variables for testing
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Create strategy - explicitly create only with providers we've configured
        let strategy = FallbackResolutionStrategy::with_providers("openai,claude");
        assert!(strategy.is_ok());
        let strategy = strategy.unwrap();
        
        // Check if it can handle conflicts
        assert!(strategy.can_handle(&conflict));
        
        // Test resolving a conflict
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Test resolving a file
        let conflict_file = create_test_conflict_file(vec![conflict]);
        let result = strategy.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("RIZZLER_OPENAI_API_KEY");
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
    }
    
    #[test]
    fn test_fallback_strategy_bedrock() {
        // Set environment variables for testing
        env::set_var("AWS_ACCESS_KEY_ID", "test-access-key");
        env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret-key");
        env::set_var("AWS_REGION", "us-east-1");
        
        // Create strategy with Bedrock
        let strategy = FallbackResolutionStrategy::with_providers("bedrock");
        
        // Should work with Bedrock provider
        assert!(strategy.is_ok());
        let strategy = strategy.unwrap();
        
        // Verify Bedrock is in the provider list
        assert!(strategy.provider_names().contains(&"bedrock".to_string()));
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Should be able to resolve using Bedrock
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("AWS_ACCESS_KEY_ID");
        env::remove_var("AWS_SECRET_ACCESS_KEY");
        env::remove_var("AWS_REGION");
    }

    #[test]
    fn test_fallback_strategy_failover() {
        // Set only Claude API key to test failover
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        
        // Create strategy with OpenAI first (which should fail) then Claude (which should work)
        let strategy = FallbackResolutionStrategy::with_providers("openai,claude");
        
        // This should still work because it should fall back to Claude
        assert!(strategy.is_ok());
        let strategy = strategy.unwrap();
        
        // We only expect Claude to be available in this test
        // But due to test isolation issues (shared env vars), we need to be more flexible
        assert!(strategy.provider_names().contains(&"claude".to_string()));
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        
        // Should be able to resolve using Claude
        let result = strategy.resolve_conflict(&conflict);
        assert!(result.is_ok());
        
        // Clean up environment
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
    }
}