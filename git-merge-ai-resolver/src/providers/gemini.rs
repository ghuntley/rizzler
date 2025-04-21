// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderConfig, AIProviderError, AIResponse, TokenUsage};
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use std::collections::HashMap;
use std::env;
use tracing::{debug, info, warn, error};

/// Google Gemini provider implementation
pub struct GeminiProvider {
    config: AIProviderConfig,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    pub fn new() -> Result<Self, AIProviderError> {
        // Get API key from environment variable
        let api_key = env::var("GIT_MERGE_GEMINI_API_KEY")
            .map_err(|_| AIProviderError::ConfigError(
                "Missing Gemini API key. Set GIT_MERGE_GEMINI_API_KEY environment variable".to_string()
            ))?;
        
        // Get other configuration from environment variables
        let model = env::var("GIT_MERGE_GEMINI_MODEL").unwrap_or_else(|_| "gemini-pro".to_string());
        let timeout_seconds = env::var("GIT_MERGE_AI_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        
        // Get custom system prompt if provided
        let system_prompt = env::var("GIT_MERGE_AI_SYSTEM_PROMPT").ok();
        
        // Create additional settings map
        let mut additional_settings = HashMap::new();
        
        // Get project ID and location
        if let Ok(project_id) = env::var("GIT_MERGE_GEMINI_PROJECT_ID") {
            additional_settings.insert("project_id".to_string(), project_id);
        }
        
        if let Ok(location) = env::var("GIT_MERGE_GEMINI_LOCATION") {
            additional_settings.insert("location".to_string(), location);
        }
        
        if let Ok(max_tokens) = env::var("GIT_MERGE_GEMINI_MAX_TOKENS") {
            additional_settings.insert("max_tokens".to_string(), max_tokens);
        }
        
        if let Ok(temperature) = env::var("GIT_MERGE_GEMINI_TEMPERATURE") {
            additional_settings.insert("temperature".to_string(), temperature);
        }
        
        Ok(GeminiProvider {
            config: AIProviderConfig {
                name: "gemini".to_string(),
                api_key,
                model,
                base_url: None, // Not typically needed for Gemini
                org_id: None,   // Gemini doesn't use org_id
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
    
    /// Parse the response from Gemini
    fn parse_response(&self, response_text: &str) -> Result<String, AIProviderError> {
        // For now, a simple implementation that just returns the text
        // In a real implementation, we would need to handle code blocks and formatting
        Ok(response_text.to_string())
    }
}

impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
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
                "Gemini provider is not available. API key is missing".to_string()
            ));
        }
        
        let system_prompt = self.create_system_prompt();
        let user_prompt = self.create_user_prompt(conflict_file, conflict);
        
        info!("Sending conflict to Gemini for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // This is a placeholder - in a real implementation, we would send the request to the Gemini API
        // and parse the response. For now, we'll just return a mock response for testing.
        
        // Mock response - this would be replaced with actual API call logic
        let mock_response = "This is a mock response from Gemini.\nIn a real implementation, we would call the Gemini API and get a real response.";
        
        let resolved_content = self.parse_response(mock_response)?;
        
        Ok(AIResponse {
            content: resolved_content,
            explanation: Some("Mock explanation from Gemini for testing".to_string()),
            token_usage: Some(TokenUsage {
                input_tokens: 110,
                output_tokens: 55,
                total_tokens: 165,
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
                "Gemini provider is not available. API key is missing".to_string()
            ));
        }
        
        let system_prompt = self.create_system_prompt();
        let user_prompt = self.create_file_prompt(conflict_file);
        
        info!("Sending entire file to Gemini for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // This is a placeholder - in a real implementation, we would send the request to the Gemini API
        // and parse the response. For now, we'll just return a mock response for testing.
        
        // Mock response - this would be replaced with actual API call logic
        let mock_response = "This is a mock response from Gemini for the entire file.\nIn a real implementation, we would call the Gemini API and get a real response.";
        
        let resolved_content = self.parse_response(mock_response)?;
        
        Ok(AIResponse {
            content: resolved_content,
            explanation: Some("Mock explanation from Gemini for testing".to_string()),
            token_usage: Some(TokenUsage {
                input_tokens: 220,
                output_tokens: 90,
                total_tokens: 310,
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
    fn test_gemini_provider_config() {
        // Set environment variables for testing
        env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
        env::set_var("GIT_MERGE_GEMINI_MODEL", "gemini-ultra");
        env::set_var("GIT_MERGE_GEMINI_PROJECT_ID", "test-project");
        env::set_var("GIT_MERGE_GEMINI_LOCATION", "us-central1");
        env::set_var("GIT_MERGE_AI_SYSTEM_PROMPT", "Test system prompt");
        env::set_var("GIT_MERGE_AI_TIMEOUT", "40");
        
        // Create provider
        let provider = GeminiProvider::new().unwrap();
        
        // Check configuration
        assert_eq!(provider.config().api_key, "test-api-key");
        assert_eq!(provider.config().model, "gemini-ultra");
        assert_eq!(provider.config().additional_settings.get("project_id"), Some(&"test-project".to_string()));
        assert_eq!(provider.config().additional_settings.get("location"), Some(&"us-central1".to_string()));
        assert_eq!(provider.config().system_prompt, Some("Test system prompt".to_string()));
        assert_eq!(provider.config().timeout_seconds, 40);
        
        // Clean up environment
        env::remove_var("GIT_MERGE_GEMINI_API_KEY");
        env::remove_var("GIT_MERGE_GEMINI_MODEL");
        env::remove_var("GIT_MERGE_GEMINI_PROJECT_ID");
        env::remove_var("GIT_MERGE_GEMINI_LOCATION");
        env::remove_var("GIT_MERGE_AI_SYSTEM_PROMPT");
        env::remove_var("GIT_MERGE_AI_TIMEOUT");
    }
    
    #[test]
    fn test_create_user_prompt() {
        // Set the API key for testing
        env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = GeminiProvider::new().unwrap();
        
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
        env::remove_var("GIT_MERGE_GEMINI_API_KEY");
    }
    
    #[test]
    fn test_resolve_conflict() {
        // Set the API key for testing
        env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = GeminiProvider::new().unwrap();
        
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
        env::remove_var("GIT_MERGE_GEMINI_API_KEY");
    }
    
    proptest! {
        #[test]
        fn test_create_user_prompt_prop(our_content in r"[\w\s]{1,100}", their_content in r"[\w\s]{1,100}") {
            // Set the API key for testing
            env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
            
            // Create a provider
            let provider = GeminiProvider::new().unwrap();
            
            // Create a test conflict
            let conflict = create_test_conflict(&our_content, &their_content);
            let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
            
            // Create user prompt
            let prompt = provider.create_user_prompt(&conflict_file, &conflict);
            
            // Check that the prompt contains the content we provided
            prop_assert!(prompt.contains(&our_content));
            prop_assert!(prompt.contains(&their_content));
            
            // Clean up environment
            env::remove_var("GIT_MERGE_GEMINI_API_KEY");
        }
    }
}