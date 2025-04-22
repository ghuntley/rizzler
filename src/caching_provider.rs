// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderError, AIResponse, AIProviderConfig};
use crate::cache::{AIResolutionCache, CacheConfig};
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use std::sync::Arc;
use tracing::info;

/// A wrapper around an AI provider that adds caching functionality
pub struct CachingAIProvider {
    /// The underlying AI provider
    provider: Box<dyn AIProvider>,
    /// The cache for AI responses
    cache: Arc<AIResolutionCache>,
}

impl CachingAIProvider {
    /// Create a new caching provider wrapping the given provider
    pub fn new(provider: Box<dyn AIProvider>) -> Self {
        // Load cache configuration from environment variables
        let config = CacheConfig::from_env();
        
        // Log cache configuration
        info!("Created CachingAIProvider with disk-based cache {} (TTL: {} hours, max entries: {:?}, auto cleanup: {}, immediate flush: {})", 
            if config.enabled { "enabled" } else { "disabled" }, 
            config.ttl_hours,
            config.max_entries,
            config.auto_cleanup,
            config.immediate_flush
        );
        
        CachingAIProvider {
            provider,
            cache: Arc::new(AIResolutionCache::from_config(config)),
        }
    }
    
    /// Create a new caching provider with explicit configuration
    pub fn with_config(provider: Box<dyn AIProvider>, config: CacheConfig) -> Self {
        // Log cache configuration
        info!("Created CachingAIProvider with disk-based cache {} (TTL: {} hours, max entries: {:?}, auto cleanup: {}, immediate flush: {})", 
            if config.enabled { "enabled" } else { "disabled" }, 
            config.ttl_hours,
            config.max_entries,
            config.auto_cleanup,
            config.immediate_flush
        );
        
        CachingAIProvider {
            provider,
            cache: Arc::new(AIResolutionCache::from_config(config)),
        }
    }
    
    /// Get a reference to the cache
    pub fn cache(&self) -> Arc<AIResolutionCache> {
        self.cache.clone()
    }
}

impl AIProvider for CachingAIProvider {
    fn name(&self) -> &str {
        // Return a name that indicates this is a caching wrapper
        &self.provider.name()
    }
    
    fn is_available(&self) -> bool {
        // The caching provider is available if the underlying provider is available
        self.provider.is_available()
    }
    
    fn config(&self) -> &AIProviderConfig {
        // Return the config of the underlying provider
        self.provider.config()
    }
    
    fn resolve_conflict(
        &self, 
        file: &ConflictFile, 
        conflict: &ConflictRegion
    ) -> Result<AIResponse, AIProviderError> {
        // Try to get from cache first
        if let Some(cached_response) = self.cache.get_conflict(conflict) {
            info!("Using cached response for conflict from model {}", cached_response.model);
            return Ok(cached_response);
        }
        
        // If not in cache, delegate to the underlying provider
        let response = self.provider.resolve_conflict(file, conflict)?;
        
        // Store in cache for future use
        self.cache.put_conflict(conflict, response.clone());
        
        Ok(response)
    }
    
    fn resolve_file(&self, file: &ConflictFile) -> Result<AIResponse, AIProviderError> {
        // Try to get from cache first
        if let Some(cached_response) = self.cache.get_file(file) {
            info!("Using cached response for file {} from model {}", 
                file.path, cached_response.model);
            return Ok(cached_response);
        }
        
        // If not in cache, delegate to the underlying provider
        let response = self.provider.resolve_file(file)?;
        
        // Store in cache for future use
        self.cache.put_file(file, response.clone());
        
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;
    
    // Mock AI Provider for testing
    struct MockAIProvider {
        name: String,
        response: String,
        config: AIProviderConfig,
    }

    impl MockAIProvider {
        fn new(name: &str, response: &str) -> Self {
            let config = AIProviderConfig {
                name: name.to_string(),
                api_key: "mock-api-key".to_string(),
                model: "mock-model".to_string(),
                base_url: None,
                org_id: None,
                system_prompt: None,
                timeout_seconds: 30,
                additional_settings: HashMap::new(),
            };
            
            MockAIProvider {
                name: name.to_string(),
                response: response.to_string(),
                config,
            }
        }
    }

    impl AIProvider for MockAIProvider {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn is_available(&self) -> bool {
            true
        }
        
        fn config(&self) -> &AIProviderConfig {
            &self.config
        }
        
        fn resolve_conflict(
            &self, 
            _file: &ConflictFile, 
            _conflict: &ConflictRegion
        ) -> Result<AIResponse, AIProviderError> {
            Ok(AIResponse {
                content: self.response.clone(),
                model: "mock-model".to_string(),
                explanation: None,
                token_usage: Some(TokenUsage {
                    input_tokens: 5,
                    output_tokens: 5,
                    total_tokens: 10,
                }),
            })
        }
        
        fn resolve_file(&self, _file: &ConflictFile) -> Result<AIResponse, AIProviderError> {
            Ok(AIResponse {
                content: self.response.clone(),
                model: "mock-model".to_string(),
                explanation: None,
                token_usage: Some(TokenUsage {
                    input_tokens: 5,
                    output_tokens: 5,
                    total_tokens: 10,
                }),
            })
        }
    }
    
    // Helper function to create a test conflict region
    fn create_test_conflict(our_content: &str, their_content: &str) -> ConflictRegion {
        ConflictRegion {
            base_content: String::from("Base content\n"),
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

    // Setup temporary directory for tests
    fn setup_test_cache() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("RIZZLER_CACHE_DIR", temp_dir.path().to_str().unwrap());
        temp_dir
    }
    
    #[test]
    fn test_caching_provider_basics() {
        let _temp_dir = setup_test_cache();
        
        // Set environment variable to enable caching
        env::set_var("RIZZLER_USE_CACHE", "true");
        
        // Create a mock provider that returns a fixed response
        let mock = Box::new(MockAIProvider::new("mock", "Resolved content\n"));
        
        // Wrap it with the caching provider
        let provider = CachingAIProvider::new(mock);
        
        // Verify the caching provider is available
        assert!(provider.is_available());
        
        // Clean up environment
        env::remove_var("RIZZLER_USE_CACHE");
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_caching_provider_conflict_resolution() {
        let _temp_dir = setup_test_cache();
        
        // Set environment variable to enable caching
        env::set_var("RIZZLER_USE_CACHE", "true");
        env::set_var("RIZZLER_CACHE_TTL_HOURS", "1");
        
        // Create a mock provider that returns a fixed response
        let mock = Box::new(MockAIProvider::new("mock", "Resolved content\n"));
        
        // Wrap it with the caching provider
        let provider = CachingAIProvider::new(mock);
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Resolve it once - should go to the underlying provider
        let response1 = provider.resolve_conflict(&file, &conflict);
        assert!(response1.is_ok());
        assert_eq!(response1.unwrap().content, "Resolved content\n");
        
        // Resolve it again - should come from cache
        let response2 = provider.resolve_conflict(&file, &conflict);
        assert!(response2.is_ok());
        assert_eq!(response2.unwrap().content, "Resolved content\n");
        
        // Clean up environment
        env::remove_var("RIZZLER_USE_CACHE");
        env::remove_var("RIZZLER_CACHE_TTL_HOURS");
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_caching_provider_file_resolution() {
        let _temp_dir = setup_test_cache();
        
        // Set environment variable to enable caching
        env::set_var("RIZZLER_USE_CACHE", "true");
        
        // Create a mock provider that returns a fixed response
        let mock = Box::new(MockAIProvider::new("mock", "Resolved file content\n"));
        
        // Wrap it with the caching provider
        let provider = CachingAIProvider::new(mock);
        
        // Create a test file
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let file = create_test_conflict_file(vec![conflict]);
        
        // Resolve it once - should go to the underlying provider
        let response1 = provider.resolve_file(&file);
        assert!(response1.is_ok());
        assert_eq!(response1.unwrap().content, "Resolved file content\n");
        
        // Resolve it again - should come from cache
        let response2 = provider.resolve_file(&file);
        assert!(response2.is_ok());
        assert_eq!(response2.unwrap().content, "Resolved file content\n");
        
        // Clean up environment
        env::remove_var("RIZZLER_USE_CACHE");
        env::remove_var("RIZZLER_CACHE_DIR");
    }
    
    #[test]
    fn test_caching_provider_disabled() {
        let _temp_dir = setup_test_cache();
        
        // Set environment variable to disable caching
        env::set_var("RIZZLER_USE_CACHE", "false");
        
        // Create a mock provider that returns a fixed response
        let mock = Box::new(MockAIProvider::new("mock", "Resolved content\n"));
        
        // Wrap it with the caching provider
        let provider = CachingAIProvider::new(mock);
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Resolve it once
        let response1 = provider.resolve_conflict(&file, &conflict);
        assert!(response1.is_ok());
        
        // Resolve it again - should still go to the underlying provider
        // since caching is disabled
        let response2 = provider.resolve_conflict(&file, &conflict);
        assert!(response2.is_ok());
        
        // Clean up environment
        env::remove_var("RIZZLER_USE_CACHE");
        env::remove_var("RIZZLER_CACHE_DIR");
    }
}