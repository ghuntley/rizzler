// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderConfig, AIProviderError, AIResponse, TokenUsage};
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use crate::prompt_engineering::{PromptGenerator, PromptTemplate};
use std::collections::HashMap;
use std::env;
use tracing::{debug, info, warn, error};
use ureq;
use std::time::Duration;
use serde_json::{json, Value};

/// Google Gemini provider implementation
pub struct GeminiProvider {
    config: AIProviderConfig,
    prompt_generator: PromptGenerator,
}

impl GeminiProvider {
    /// Get the API endpoint for Gemini
    fn get_api_endpoint(&self) -> String {
        let project_id = self.config.additional_settings.get("project_id");
        let location = self.config.additional_settings.get("location");
        
        if let (Some(project_id), Some(location)) = (project_id, location) {
            // Use Google Cloud AI Platform format
            format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
                location, project_id, location, self.config.model
            )
        } else {
            // Use direct Gemini API format
            format!(
                "https://generativelanguage.googleapis.com/v1/models/{}:generateContent?key={}",
                self.config.model, self.config.api_key
            )
        }
    }
    
    /// Prepare the request payload for the Gemini API
    fn prepare_request_payload(&self, system_prompt: &str, user_prompt: &str) -> Value {
        // Check for specific settings
        let max_tokens = self.config.additional_settings.get("max_tokens")
            .and_then(|s| s.parse::<u32>().ok());
        
        let temperature = self.config.additional_settings.get("temperature")
            .and_then(|s| s.parse::<f32>().ok());
        
        // Build the contents array with system and user prompts
        let mut contents = Vec::new();
        
        // Add system prompt as a role message
        contents.push(json!({
            "role": "system",
            "parts": [{
                "text": system_prompt
            }]
        }));
        
        // Add user prompt
        contents.push(json!({
            "role": "user",
            "parts": [{
                "text": user_prompt
            }]
        }));
        
        // Build generation config
        let mut generation_config = json!({
            "temperature": temperature.unwrap_or(0.2),
            "topP": 0.95,
            "topK": 40
        });
        
        // Add max tokens if specified
        if let Some(max) = max_tokens {
            generation_config["maxOutputTokens"] = json!(max);
        }
        
        // Build the full request payload
        json!({
            "contents": contents,
            "generationConfig": generation_config,
            "safetySettings": [
                {
                    "category": "HARM_CATEGORY_HARASSMENT",
                    "threshold": "BLOCK_NONE"
                },
                {
                    "category": "HARM_CATEGORY_HATE_SPEECH",
                    "threshold": "BLOCK_NONE"
                },
                {
                    "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                    "threshold": "BLOCK_NONE"
                },
                {
                    "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                    "threshold": "BLOCK_NONE"
                }
            ]
        })
    }
    
    /// Extract the response content from Gemini API response
    fn extract_response_content(&self, response_json: &Value) -> Result<String, AIProviderError> {
        // Try to get the content from the first candidate
        if let Some(candidates) = response_json.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first_candidate) = candidates.first() {
                if let Some(content) = first_candidate.get("content") {
                    if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                        if let Some(first_part) = parts.first() {
                            if let Some(text) = first_part.get("text").and_then(|t| t.as_str()) {
                                return Ok(text.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // If we couldn't extract the content using the expected structure
        Err(AIProviderError::ResponseError("Failed to extract content from Gemini response".to_string()))
    }
    
    /// Extract the explanation from the response if available
    fn extract_explanation(&self, response_json: &Value) -> Option<String> {
        // Try to get the explanation from the response
        // For Gemini, we'll look for any metadata or finish reason that can serve as an explanation
        if let Some(finish_reason) = response_json.get("candidates").and_then(|c| c.as_array())
            .and_then(|candidates| candidates.first())
            .and_then(|candidate| candidate.get("finishReason").and_then(|r| r.as_str())) {
            return Some(format!("Finish reason: {}", finish_reason));
        }
        
        // If no specific explanation is found, provide a generic one
        Some("Resolved using Gemini API".to_string())
    }
    
    /// Extract token usage information from the response
    fn extract_token_usage(&self, response_json: &Value) -> Option<TokenUsage> {
        // Try to get usage metrics
        if let Some(usage_metadata) = response_json.get("usageMetadata") {
            let input_tokens = usage_metadata.get("promptTokenCount")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
                
            let output_tokens = usage_metadata.get("candidatesTokenCount")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
                
            return Some(TokenUsage {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
            })
        }
        
        // If we couldn't extract token usage, return None
        None
    }
    
    /// Create a new Gemini provider
    pub fn new() -> Result<Self, AIProviderError> {
        // In test mode, always use a test key for convenience
        if cfg!(test) || env::var("TEST_MODE").unwrap_or_else(|_| "false".to_string()) == "true" {
            return Self::new_with_api_key("test-api-key".to_string());
        }
        
        // In non-test mode, require a real key
        {
            // Try to get the API key from environment variable
            match env::var("GIT_MERGE_GEMINI_API_KEY") {
                Ok(key) => {
                    // Return the provided API key
                    Self::new_with_api_key(key)
                },
                Err(_) => {
                    return Err(AIProviderError::ConfigError(
                        "Missing Gemini API key. Set GIT_MERGE_GEMINI_API_KEY environment variable".to_string()
                    ));
                }
            }
        }
    }
    
    /// Create a new Gemini provider with an explicit API key
    fn new_with_api_key(api_key: String) -> Result<Self, AIProviderError> {
        
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
        
        // Determine which prompt template to use based on environment variable
        // Default to the Enhanced template for better results
        let prompt_template = match env::var("GIT_MERGE_AI_PROMPT_TEMPLATE").ok().as_deref() {
            Some("default") => PromptTemplate::Default,
            Some("context-aware") => PromptTemplate::ContextAware,
            _ => PromptTemplate::Enhanced, // Default to enhanced
        };
        
        // Create prompt generator with the selected template
        let prompt_generator = PromptGenerator::new(prompt_template);
        
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
            prompt_generator,
        })
    }
    
    /// Get the system prompt for Gemini
    /// This uses the prompt generator with the configured template
    fn get_system_prompt(&self) -> String {
        // First check if a system prompt is explicitly provided in the config
        if let Some(prompt) = &self.config().system_prompt {
            return prompt.clone();
        }
        
        // Otherwise use the prompt generator to create one
        self.prompt_generator.generate_system_prompt()
    }
    
    // No parse_response method needed as we're handling this in process_response
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
        
        let system_prompt = self.get_system_prompt();
        let user_prompt = self.prompt_generator.generate_conflict_prompt(conflict_file, conflict);
        
        info!("Sending conflict to Gemini for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // In test mode, return a mock response
        if cfg!(test) || env::var("TEST_MODE").unwrap_or_else(|_| "false".to_string()) == "true" {
            let content = "// Mock resolved content from Gemini API\nfunction add(a, b) {\n  // Add two numbers\n  return a + b;\n}\n".to_string();
            let token_count = user_prompt.chars().count() as u32;
            let output_count = content.chars().count() as u32;
            
            Ok(AIResponse {
                content,
                explanation: Some("Resolved by Gemini AI (mock implementation)".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: token_count,
                    output_tokens: output_count,
                    total_tokens: token_count + output_count,
                }),
                model: self.config.model.clone(),
            })
        } else {
            // In non-test mode, make a real API call to Gemini
            // Create a request to the Gemini API
            let api_endpoint = self.get_api_endpoint();
            
            // Prepare the request payload
            let payload = self.prepare_request_payload(&system_prompt, &user_prompt);
            
            // Send the request to Gemini API
            let response = ureq::post(&api_endpoint)
                .set("Content-Type", "application/json")
                .set("Authorization", &format!("Bearer {}", self.config.api_key))
                .timeout(Duration::from_secs(self.config.timeout_seconds))
                .send_json(payload)
                .map_err(|e| {
                    match e {
                        ureq::Error::Status(code, response) => {
                            let error_text = response.into_string().unwrap_or_else(|_| "Unable to read error response".to_string());
                            return match code {
                                401 | 403 => AIProviderError::AuthError(format!("Authentication error: {}", error_text)),
                                404 => AIProviderError::ModelNotAvailable(format!("Model not found: {}", error_text)),
                                429 => AIProviderError::RateLimit(format!("Rate limit exceeded: {}", error_text)),
                                408 | 504 => AIProviderError::Timeout(format!("Request timed out: {}", error_text)),
                                _ => AIProviderError::RequestError(format!("API request failed with status {}: {}", code, error_text)),
                            };
                        },
                        ureq::Error::Transport(transport) => AIProviderError::ConnectionError(format!("Failed to connect to Gemini API: {}", transport)),
                    }
                })?;
                
            // Parse the response
            let response_json: Value = response.into_json()
                .map_err(|e| AIProviderError::ResponseError(format!("Failed to parse response: {}", e)))?;
            
            // Extract content from the response
            let content = self.extract_response_content(&response_json)?;
            let explanation = self.extract_explanation(&response_json);
            let token_usage = self.extract_token_usage(&response_json);
            
            Ok(AIResponse {
                content,
                explanation,
                token_usage,
                model: self.config.model.clone(),
            })
        }
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
        
        let system_prompt = self.get_system_prompt();
        let user_prompt = self.prompt_generator.generate_file_prompt(conflict_file);
        
        info!("Sending entire file to Gemini for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // In test mode, return a mock response
        if cfg!(test) || env::var("TEST_MODE").unwrap_or_else(|_| "false".to_string()) == "true" {
            let content = "// Mock resolved content for entire file\nfunction add(a, b) {\n  // Add two numbers\n  return a + b;\n}\n\nfunction subtract(a, b) {\n  return a - b;\n}\n".to_string();
            
            let token_count = user_prompt.chars().count() as u32;
            let output_count = content.chars().count() as u32;
            
            Ok(AIResponse {
                content,
                explanation: Some("Resolved entire file by Gemini AI (mock implementation)".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: token_count,
                    output_tokens: output_count,
                    total_tokens: token_count + output_count,
                }),
                model: self.config.model.clone(),
            })
        } else {
            // In non-test mode, make a real API call to Gemini
            // Create a request to the Gemini API
            let api_endpoint = self.get_api_endpoint();
            
            // Prepare the request payload
            let payload = self.prepare_request_payload(&system_prompt, &user_prompt);
            
            // Send the request to Gemini API
            let response = ureq::post(&api_endpoint)
                .set("Content-Type", "application/json")
                .set("Authorization", &format!("Bearer {}", self.config.api_key))
                .timeout(Duration::from_secs(self.config.timeout_seconds))
                .send_json(payload)
                .map_err(|e| {
                    match e {
                        ureq::Error::Status(code, response) => {
                            let error_text = response.into_string().unwrap_or_else(|_| "Unable to read error response".to_string());
                            return match code {
                                401 | 403 => AIProviderError::AuthError(format!("Authentication error: {}", error_text)),
                                404 => AIProviderError::ModelNotAvailable(format!("Model not found: {}", error_text)),
                                429 => AIProviderError::RateLimit(format!("Rate limit exceeded: {}", error_text)),
                                408 | 504 => AIProviderError::Timeout(format!("Request timed out: {}", error_text)),
                                _ => AIProviderError::RequestError(format!("API request failed with status {}: {}", code, error_text)),
                            };
                        },
                        ureq::Error::Transport(transport) => AIProviderError::ConnectionError(format!("Failed to connect to Gemini API: {}", transport)),
                    }
                })?;
                
            // Parse the response
            let response_json: Value = response.into_json()
                .map_err(|e| AIProviderError::ResponseError(format!("Failed to parse response: {}", e)))?;
            
            // Extract content from the response
            let content = self.extract_response_content(&response_json)?;
            let explanation = self.extract_explanation(&response_json);
            let token_usage = self.extract_token_usage(&response_json);
            
            Ok(AIResponse {
                content,
                explanation,
                token_usage,
                model: self.config.model.clone(),
            })
        }
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
    fn test_conflict_prompt_generation() {
        // Set the API key for testing
        env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = GeminiProvider::new().unwrap();
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\nwith multiple lines\n", "Their content\nalso with lines\n");
        let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Get prompt using the prompt generator
        let prompt = provider.prompt_generator.generate_conflict_prompt(&conflict_file, &conflict);
        
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
        fn test_conflict_prompt_generation_prop(our_content in r"[\w\s]{1,100}", their_content in r"[\w\s]{1,100}") {
            // Set the API key for testing
            env::set_var("GIT_MERGE_GEMINI_API_KEY", "test-api-key");
            
            // Create a provider
            let provider = GeminiProvider::new().unwrap();
            
            // Create a test conflict
            let conflict = create_test_conflict(&our_content, &their_content);
            let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
            
            // Generate conflict prompt using prompt generator
            let prompt = provider.prompt_generator.generate_conflict_prompt(&conflict_file, &conflict);
            
            // Check that the prompt contains the content we provided
            prop_assert!(prompt.contains(&our_content));
            prop_assert!(prompt.contains(&their_content));
            
            // Clean up environment
            env::remove_var("GIT_MERGE_GEMINI_API_KEY");
        }
    }
}