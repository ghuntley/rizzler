// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderError, AIResponse};
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use std::env;
use std::thread;
use std::time::Duration;
use tracing::{debug, info, warn, error};

/// Default maximum number of retries if not specified
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default initial backoff time in milliseconds
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 1000; // 1 second

/// Default maximum backoff time in milliseconds
const DEFAULT_MAX_BACKOFF_MS: u64 = 30000; // 30 seconds

/// Default backoff multiplier (for exponential backoff)
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

/// Default jitter factor to add randomness to backoff times
const DEFAULT_JITTER_FACTOR: f64 = 0.1;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Initial backoff time in milliseconds
    pub initial_backoff_ms: u64,
    
    /// Maximum backoff time in milliseconds
    pub max_backoff_ms: u64,
    
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    
    /// Jitter factor to add randomness to backoff times
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        // Create default config from constants
        let mut config = RetryConfig {
            max_retries: DEFAULT_MAX_RETRIES,
            initial_backoff_ms: DEFAULT_INITIAL_BACKOFF_MS,
            max_backoff_ms: DEFAULT_MAX_BACKOFF_MS,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            jitter_factor: DEFAULT_JITTER_FACTOR,
        };
        
        // Override from environment variables if available
        if let Ok(max_retries) = env::var("RIZZLER_MAX_RETRIES") {
            if let Ok(value) = max_retries.parse::<u32>() {
                config.max_retries = value;
            }
        }
        
        if let Ok(initial_backoff) = env::var("RIZZLER_INITIAL_BACKOFF_MS") {
            if let Ok(value) = initial_backoff.parse::<u64>() {
                config.initial_backoff_ms = value;
            }
        }
        
        if let Ok(max_backoff) = env::var("RIZZLER_MAX_BACKOFF_MS") {
            if let Ok(value) = max_backoff.parse::<u64>() {
                config.max_backoff_ms = value;
            }
        }
        
        if let Ok(multiplier) = env::var("RIZZLER_BACKOFF_MULTIPLIER") {
            if let Ok(value) = multiplier.parse::<f64>() {
                config.backoff_multiplier = value;
            }
        }
        
        if let Ok(jitter) = env::var("RIZZLER_JITTER_FACTOR") {
            if let Ok(value) = jitter.parse::<f64>() {
                config.jitter_factor = value;
            }
        }
        
        config
    }
}

impl RetryConfig {
    /// Calculate backoff time for a specific retry attempt
    pub fn calculate_backoff_time(&self, retry_attempt: u32) -> Duration {
        // Use exponential backoff formula
        let base_backoff = self.initial_backoff_ms as f64 * 
            self.backoff_multiplier.powf(retry_attempt as f64);
            
        // Cap at maximum backoff
        let capped_backoff = base_backoff.min(self.max_backoff_ms as f64);
        
        // Add jitter to avoid thundering herd problem
        let jitter_range = capped_backoff * self.jitter_factor;
        let jitter = rand::random::<f64>() * jitter_range * 2.0 - jitter_range;
        
        let backoff_with_jitter = (capped_backoff + jitter).max(0.0) as u64;
        
        Duration::from_millis(backoff_with_jitter)
    }
    
    /// Determine if an error is retryable
    pub fn is_retryable_error(&self, error: &AIProviderError) -> bool {
        match error {
            AIProviderError::ConnectionError(_) => true,
            AIProviderError::RequestError(_) => true,
            AIProviderError::Timeout(_) => true,
            AIProviderError::RateLimit(_) => true,
            _ => false,
        }
    }
}

/// A wrapper around an AIProvider that adds retry capability
pub struct RetryableProvider {
    /// The underlying AI provider
    provider: Box<dyn AIProvider>,
    
    /// Retry configuration
    config: RetryConfig,
}

impl RetryableProvider {
    /// Create a new retryable provider with default retry configuration
    pub fn new(provider: Box<dyn AIProvider>) -> Self {
        RetryableProvider {
            provider,
            config: RetryConfig::default(),
        }
    }
    
    /// Create a new retryable provider with custom retry configuration
    pub fn with_config(provider: Box<dyn AIProvider>, config: RetryConfig) -> Self {
        RetryableProvider {
            provider,
            config,
        }
    }
    
    /// Resolve a conflict with automatic retries
    pub fn resolve_conflict_with_retries(
        &self,
        conflict_file: &ConflictFile,
        conflict: &ConflictRegion,
    ) -> Result<AIResponse, AIProviderError> {
        let mut last_error = None;
        
        // Try up to max_retries + 1 times (original attempt + retries)
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                info!("Retry attempt {} of {} for provider {}", 
                      attempt, self.config.max_retries, self.provider.name());
            }
            
            match self.provider.resolve_conflict(conflict_file, conflict) {
                Ok(response) => {
                    if attempt > 0 {
                        info!("Successfully resolved conflict after {} retries", attempt);
                    }
                    return Ok(response);
                },
                Err(err) => {
                    if attempt < self.config.max_retries && self.config.is_retryable_error(&err) {
                        // Calculate backoff time for this attempt
                        let backoff = self.config.calculate_backoff_time(attempt);
                        
                        warn!("Provider {} failed with retryable error: {}", 
                              self.provider.name(), err);
                        warn!("Retrying in {:?}", backoff);
                        
                        // Sleep before retrying
                        thread::sleep(backoff);
                        last_error = Some(err);
                    } else if !self.config.is_retryable_error(&err) {
                        // Non-retryable error, fail immediately
                        error!("Provider {} failed with non-retryable error: {}", 
                               self.provider.name(), err);
                        return Err(err);
                    } else {
                        // We've reached max retries
                        error!("Provider {} failed after {} retries: {}", 
                               self.provider.name(), self.config.max_retries, err);
                        last_error = Some(err);
                    }
                }
            }
        }
        
        // If we get here, all retries failed
        Err(last_error.unwrap_or_else(|| {
            AIProviderError::RequestError("All retries failed with unknown error".to_string())
        }))
    }
    
    /// Resolve a file with automatic retries
    pub fn resolve_file_with_retries(
        &self,
        conflict_file: &ConflictFile,
    ) -> Result<AIResponse, AIProviderError> {
        let mut last_error = None;
        
        // Try up to max_retries + 1 times (original attempt + retries)
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                info!("Retry attempt {} of {} for provider {}", 
                      attempt, self.config.max_retries, self.provider.name());
            }
            
            match self.provider.resolve_file(conflict_file) {
                Ok(response) => {
                    if attempt > 0 {
                        info!("Successfully resolved file after {} retries", attempt);
                    }
                    return Ok(response);
                },
                Err(err) => {
                    if attempt < self.config.max_retries && self.config.is_retryable_error(&err) {
                        // Calculate backoff time for this attempt
                        let backoff = self.config.calculate_backoff_time(attempt);
                        
                        warn!("Provider {} failed with retryable error: {}", 
                              self.provider.name(), err);
                        warn!("Retrying in {:?}", backoff);
                        
                        // Sleep before retrying
                        thread::sleep(backoff);
                        last_error = Some(err);
                    } else if !self.config.is_retryable_error(&err) {
                        // Non-retryable error, fail immediately
                        error!("Provider {} failed with non-retryable error: {}", 
                               self.provider.name(), err);
                        return Err(err);
                    } else {
                        // We've reached max retries
                        error!("Provider {} failed after {} retries: {}", 
                               self.provider.name(), self.config.max_retries, err);
                        last_error = Some(err);
                    }
                }
            }
        }
        
        // If we get here, all retries failed
        Err(last_error.unwrap_or_else(|| {
            AIProviderError::RequestError("All retries failed with unknown error".to_string())
        }))
    }
}

impl AIProvider for RetryableProvider {
    fn name(&self) -> &str {
        // Forward to the wrapped provider
        self.provider.name()
    }
    
    fn is_available(&self) -> bool {
        // Forward to the wrapped provider
        self.provider.is_available()
    }
    
    fn config(&self) -> &crate::ai_provider::AIProviderConfig {
        // Forward to the wrapped provider
        self.provider.config()
    }
    
    fn resolve_conflict(
        &self,
        conflict_file: &ConflictFile,
        conflict: &ConflictRegion,
    ) -> Result<AIResponse, AIProviderError> {
        // Use our retry logic
        self.resolve_conflict_with_retries(conflict_file, conflict)
    }
    
    fn resolve_file(
        &self,
        conflict_file: &ConflictFile,
    ) -> Result<AIResponse, AIProviderError> {
        // Use our retry logic
        self.resolve_file_with_retries(conflict_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_provider::{AIProviderConfig, TokenUsage};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    
    // Mock AI provider for testing
    struct MockProvider {
        name: String,
        available: bool,
        config: AIProviderConfig,
        fail_count: Rc<RefCell<u32>>,
        failure_type: AIProviderError,
    }
    
    impl MockProvider {
        fn new(name: &str, available: bool, fail_count: u32, failure_type: AIProviderError) -> Self {
            MockProvider {
                name: name.to_string(),
                available,
                config: AIProviderConfig {
                    name: name.to_string(),
                    api_key: "test-key".to_string(),
                    model: "test-model".to_string(),
                    base_url: None,
                    org_id: None,
                    system_prompt: None,
                    timeout_seconds: 60,
                    additional_settings: HashMap::new(),
                },
                fail_count: Rc::new(RefCell::new(fail_count)),
                failure_type,
            }
        }
    }
    
    impl AIProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn is_available(&self) -> bool {
            self.available
        }
        
        fn config(&self) -> &AIProviderConfig {
            &self.config
        }
        
        fn resolve_conflict(
            &self,
            _conflict_file: &ConflictFile,
            _conflict: &ConflictRegion,
        ) -> Result<AIResponse, AIProviderError> {
            let mut fail_count = self.fail_count.borrow_mut();
            
            if *fail_count > 0 {
                *fail_count -= 1;
                return Err(self.failure_type.clone());
            }
            
            Ok(AIResponse {
                content: "Resolved content".to_string(),
                explanation: Some("Test explanation".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    total_tokens: 15,
                }),
                model: "test-model".to_string(),
            })
        }
        
        fn resolve_file(
            &self,
            _conflict_file: &ConflictFile,
        ) -> Result<AIResponse, AIProviderError> {
            let mut fail_count = self.fail_count.borrow_mut();
            
            if *fail_count > 0 {
                *fail_count -= 1;
                return Err(self.failure_type.clone());
            }
            
            Ok(AIResponse {
                content: "Resolved content".to_string(),
                explanation: Some("Test explanation".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    total_tokens: 15,
                }),
                model: "test-model".to_string(),
            })
        }
    }
    
    // Helper function to create a test conflict region
    fn create_test_conflict() -> ConflictRegion {
        ConflictRegion {
            base_content: String::new(),
            our_content: "Our content\n".to_string(),
            their_content: "Their content\n".to_string(),
            start_line: 1,
            end_line: 5,
        }
    }
    
    // Helper function to create a test conflict file
    fn create_test_conflict_file() -> ConflictFile {
        ConflictFile {
            path: "test.txt".to_string(),
            conflicts: vec![create_test_conflict()],
            content: "<<<<<<< HEAD\nOur content\n=======\nTheir content\n>>>>>>> branch-name\n".to_string(),
        }
    }
    
    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        
        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(config.initial_backoff_ms, DEFAULT_INITIAL_BACKOFF_MS);
        assert_eq!(config.max_backoff_ms, DEFAULT_MAX_BACKOFF_MS);
        assert_eq!(config.backoff_multiplier, DEFAULT_BACKOFF_MULTIPLIER);
        assert_eq!(config.jitter_factor, DEFAULT_JITTER_FACTOR);
    }
    
    #[test]
    fn test_retry_config_from_env() {
        // Set environment variables
        env::set_var("RIZZLER_MAX_RETRIES", "5");
        env::set_var("RIZZLER_INITIAL_BACKOFF_MS", "500");
        env::set_var("RIZZLER_MAX_BACKOFF_MS", "10000");
        env::set_var("RIZZLER_BACKOFF_MULTIPLIER", "1.5");
        env::set_var("RIZZLER_JITTER_FACTOR", "0.2");
        
        let config = RetryConfig::default();
        
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_backoff_ms, 500);
        assert_eq!(config.max_backoff_ms, 10000);
        assert_eq!(config.backoff_multiplier, 1.5);
        assert_eq!(config.jitter_factor, 0.2);
        
        // Clean up environment
        env::remove_var("RIZZLER_MAX_RETRIES");
        env::remove_var("RIZZLER_INITIAL_BACKOFF_MS");
        env::remove_var("RIZZLER_MAX_BACKOFF_MS");
        env::remove_var("RIZZLER_BACKOFF_MULTIPLIER");
        env::remove_var("RIZZLER_JITTER_FACTOR");
    }
    
    #[test]
    fn test_is_retryable_error() {
        let config = RetryConfig::default();
        
        // Retryable errors
        assert!(config.is_retryable_error(&AIProviderError::ConnectionError("test".to_string())));
        assert!(config.is_retryable_error(&AIProviderError::RequestError("test".to_string())));
        assert!(config.is_retryable_error(&AIProviderError::Timeout("test".to_string())));
        assert!(config.is_retryable_error(&AIProviderError::RateLimit("test".to_string())));
        
        // Non-retryable errors
        assert!(!config.is_retryable_error(&AIProviderError::AuthError("test".to_string())));
        assert!(!config.is_retryable_error(&AIProviderError::ModelNotAvailable("test".to_string())));
        assert!(!config.is_retryable_error(&AIProviderError::PromptError("test".to_string())));
        assert!(!config.is_retryable_error(&AIProviderError::ConfigError("test".to_string())));
        assert!(!config.is_retryable_error(&AIProviderError::ResponseError("test".to_string())));
    }
    
    #[test]
    fn test_calculate_backoff_time() {
        let config = RetryConfig {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 1000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // No jitter for deterministic testing
        };
        
        // First retry: 100ms * 2^0 = 100ms
        assert_eq!(config.calculate_backoff_time(0), Duration::from_millis(100));
        
        // Second retry: 100ms * 2^1 = 200ms
        assert_eq!(config.calculate_backoff_time(1), Duration::from_millis(200));
        
        // Third retry: 100ms * 2^2 = 400ms
        assert_eq!(config.calculate_backoff_time(2), Duration::from_millis(400));
        
        // Fourth retry: 100ms * 2^3 = 800ms
        assert_eq!(config.calculate_backoff_time(3), Duration::from_millis(800));
        
        // Fifth retry: would be 1600ms, but capped at 1000ms
        assert_eq!(config.calculate_backoff_time(4), Duration::from_millis(1000));
    }
    
    #[test]
    fn test_retryable_provider_succeeds_first_try() {
        // Create a mock provider that never fails
        let mock_provider = MockProvider::new(
            "test", 
            true, 
            0, // Fail count: 0 means never fails
            AIProviderError::ConnectionError("test".to_string())
        );
        
        // Create a retryable provider with default config
        let retryable_provider = RetryableProvider::new(Box::new(mock_provider));
        
        // Create test conflict and file
        let conflict = create_test_conflict();
        let conflict_file = create_test_conflict_file();
        
        // Test resolve_conflict
        let result = retryable_provider.resolve_conflict(&conflict_file, &conflict);
        assert!(result.is_ok());
        
        // Test resolve_file
        let result = retryable_provider.resolve_file(&conflict_file);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_retryable_provider_succeeds_after_retries() {
        // Create a mock provider that fails twice then succeeds
        let mock_provider = MockProvider::new(
            "test", 
            true, 
            2, // Fail count: 2 means fails twice then succeeds
            AIProviderError::ConnectionError("test".to_string())
        );
        
        // Create a retryable provider with faster retry config for testing
        let config = RetryConfig {
            max_retries: 3,
            initial_backoff_ms: 1,  // 1ms for fast tests
            max_backoff_ms: 10,     // 10ms for fast tests
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,      // No jitter for deterministic testing
        };
        
        let retryable_provider = RetryableProvider::with_config(
            Box::new(mock_provider),
            config
        );
        
        // Create test conflict and file
        let conflict = create_test_conflict();
        let conflict_file = create_test_conflict_file();
        
        // Test resolve_conflict with retries
        let result = retryable_provider.resolve_conflict(&conflict_file, &conflict);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_retryable_provider_all_retries_fail() {
        // Create a mock provider that always fails
        let mock_provider = MockProvider::new(
            "test", 
            true, 
            999, // Fail count: very large number so it always fails
            AIProviderError::ConnectionError("test".to_string())
        );
        
        // Create a retryable provider with faster retry config for testing
        let config = RetryConfig {
            max_retries: 2,          // Only retry twice
            initial_backoff_ms: 1,   // 1ms for fast tests
            max_backoff_ms: 10,      // 10ms for fast tests
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,      // No jitter for deterministic testing
        };
        
        let retryable_provider = RetryableProvider::with_config(
            Box::new(mock_provider),
            config
        );
        
        // Create test conflict and file
        let conflict = create_test_conflict();
        let conflict_file = create_test_conflict_file();
        
        // Test resolve_conflict with retries that all fail
        let result = retryable_provider.resolve_conflict(&conflict_file, &conflict);
        assert!(result.is_err());
        
        // Check error type
        match result {
            Err(AIProviderError::ConnectionError(_)) => {}, // Expected
            _ => panic!("Expected ConnectionError, got {:?}", result),
        }
    }
    
    #[test]
    fn test_non_retryable_error_fails_immediately() {
        // Create a mock provider that fails with a non-retryable error
        let mock_provider = MockProvider::new(
            "test", 
            true, 
            999, // Fail count: very large number so it always fails
            AIProviderError::AuthError("test".to_string()) // Not retryable
        );
        
        // Create a retryable provider
        let retryable_provider = RetryableProvider::new(Box::new(mock_provider));
        
        // Create test conflict and file
        let conflict = create_test_conflict();
        let conflict_file = create_test_conflict_file();
        
        // Test resolve_conflict should fail immediately without retries
        let result = retryable_provider.resolve_conflict(&conflict_file, &conflict);
        assert!(result.is_err());
        
        // Check error type
        match result {
            Err(AIProviderError::AuthError(_)) => {}, // Expected
            _ => panic!("Expected AuthError, got {:?}", result),
        }
    }
}