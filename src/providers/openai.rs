// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderConfig, AIProviderError, AIResponse, TokenUsage};
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use crate::prompt_engineering::{PromptGenerator, PromptTemplate};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tracing::{debug, info};
use ureq;
use serde_json;

/// OpenAI provider implementation
pub struct OpenAIProvider {
    config: AIProviderConfig,
    prompt_generator: PromptGenerator,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new() -> Result<Self, AIProviderError> {
        // Get API key from environment variable
        let api_key = env::var("RIZZLER_OPENAI_API_KEY")
            .map_err(|_| AIProviderError::ConfigError(
                "Missing OpenAI API key. Set RIZZLER_OPENAI_API_KEY environment variable".to_string()
            ))?;
        
        // Get other configuration from environment variables
        let base_url = env::var("RIZZLER_OPENAI_BASE_URL").ok();
        let org_id = env::var("RIZZLER_OPENAI_ORG_ID").ok();
        let model = env::var("RIZZLER_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4-turbo".to_string());
        let timeout_seconds = env::var("RIZZLER_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        
        // Get custom system prompt if provided
        let system_prompt = env::var("RIZZLER_SYSTEM_PROMPT").ok();
        
        // Create additional settings map
        let mut additional_settings = HashMap::new();
        if let Ok(max_tokens) = env::var("RIZZLER_OPENAI_MAX_TOKENS") {
            additional_settings.insert("max_tokens".to_string(), max_tokens);
        }
        
        // Determine which prompt template to use based on environment variable
        // Default to the Enhanced template for better results
        let prompt_template = match env::var("RIZZLER_PROMPT_TEMPLATE").ok().as_deref() {
            Some("default") => PromptTemplate::Default,
            Some("context-aware") => PromptTemplate::ContextAware,
            _ => PromptTemplate::Enhanced, // Default to enhanced
        };
        
        // Create prompt generator with the selected template
        let prompt_generator = PromptGenerator::new(prompt_template);
        
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
            prompt_generator,
        })
    }
    
    /// Get the system prompt for OpenAI
    fn get_system_prompt(&self) -> String {
        // First check if a system prompt is explicitly provided in the config
        if let Some(prompt) = &self.config().system_prompt {
            return prompt.clone();
        }
        
        // Otherwise use the prompt generator to create one
        self.prompt_generator.generate_system_prompt()
    }
    
    /// Extract response content from the OpenAI API response JSON
    fn extract_response_content(&self, response_json: &serde_json::Value) -> Result<String, AIProviderError> {
        // Extract the response content from the choices array
        if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(message) = first_choice.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        return Ok(content.to_string());
                    }
                }
            }
        }
        
        // If we couldn't extract the content using the expected structure
        Err(AIProviderError::ResponseError("Failed to extract content from OpenAI response".to_string()))
    }
    
    /// Extract token usage information from the response
    fn extract_token_usage(&self, response_json: &serde_json::Value) -> Option<TokenUsage> {
        if let Some(usage) = response_json.get("usage") {
            let prompt_tokens = usage.get("prompt_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
                
            let completion_tokens = usage.get("completion_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
                
            let total_tokens = usage.get("total_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
                
            return Some(TokenUsage {
                input_tokens: prompt_tokens,
                output_tokens: completion_tokens,
                total_tokens,
            });
        }
        
        None
    }
    
    /// Parse the response from OpenAI
    fn parse_response(&self, response_text: &str) -> Result<String, AIProviderError> {
        // Extract code blocks if present (common in OpenAI responses)
        let re = regex::Regex::new(r"```(?:\w+)?\s*([\s\S]*?)```").ok();
        
        if let Some(re) = re {
            let captures: Vec<_> = re.captures_iter(response_text).collect();
            
            if !captures.is_empty() {
                // If we have code blocks, extract the content from the first one
                if let Some(capture) = captures.first() {
                    if capture.len() > 1 {
                        return Ok(capture[1].trim().to_string());
                    }
                }
            }
        }
        
        // If no code blocks or regex failed, just clean the text
        // Remove any explanations, comments, etc. that may be outside the solved code
        let cleaned_text = response_text.trim();
        Ok(cleaned_text.to_string())
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
        
        let system_prompt = self.get_system_prompt();
        let user_prompt = self.prompt_generator.generate_conflict_prompt(conflict_file, conflict);
        
        info!("Sending conflict to OpenAI for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // If we're in test mode, return a mock response
        if cfg!(test) || env::var("TEST_MODE").unwrap_or_else(|_| "false".to_string()) == "true" {
            // For testing with the example merge conflicts file
            let is_merge_example = conflict_file.path.contains("merge_conflicts_example.sh");
            
            let content = if is_merge_example {
                // Check which conflict we're resolving based on content
                if conflict.our_content.contains("DB_HOST=\"primary.db.example.com\"") {
                    // Database settings conflict
                    "DB_HOST=\"replica.db.example.com\" # Using replica from feature/app-metrics\nDB_PORT=5432\nDB_USER=\"app_user\"\nDB_PASSWORD=\"new_very_secure_password\" # Using newer password from feature/app-metrics\nDB_NAME=\"production_db\"".to_string()
                } else if conflict.our_content.contains("check_dependencies()") {
                    // Check dependencies conflict
                    "check_dependencies() {\n    echo \"Checking dependencies...\"\n    for dep in \"curl\" \"jq\" \"wget\"; do\n        if ! command -v $dep &> /dev/null; then\n            install_dependency $dep\n        fi\n    done\n}\n\ninstall_dependency() {\n    echo \"Installing $1...\"\n    # Implementation details\n}".to_string()
                } else if conflict.our_content.contains("handle_error()") {
                    // Handle error conflict
                    "handle_error() {\n    echo \"Error: $1\"\n    exit 1\n}\n\n# Main application function\nmain() {\n    # Parse command line arguments\n    parse_arguments \"$@\"\n    \n    # Initialize the application\n    check_dependencies\n    setup_database_connection\n    setup_cache\n    initialize_metrics\n    \n    # Start application\n    echo \"Starting application with $(get_thread_count) threads...\"\n    start_worker_processes\n    setup_signal_handlers\n    wait_for_completion\n}\n\nparse_arguments() {\n    # Parse command line arguments\n    while [[ $# -gt 0 ]]; do\n        case $1 in\n            --debug) DEBUG_MODE=true ;;\n            --threads=*) THREAD_COUNT=\"${1#*=}\" ;;\n            *) echo \"Unknown option: $1\" ;;\n        esac\n        shift\n    done\n}\n\nget_thread_count() {\n    echo ${THREAD_COUNT:-$(nproc)}\n}".to_string()
                } else if conflict.our_content.contains("main") && conflict.their_content.contains("main \"$@\"") {
                    // Main function call conflict
                    "# Call main function with arguments\nmain \"$@\"".to_string()
                } else {
                    // Default mock response
                    "// Default mock resolved content from OpenAI API\nfunction example() {\n    // Combined implementation\n    console.log('Resolved content');\n}\n".to_string()
                }
            } else {
                // Default mock response for non-example files
                "// Mock resolved content from OpenAI API\nfunction example() {\n    // Combined implementation\n    console.log('Resolved content');\n}\n".to_string()
            };
            
            let token_count = user_prompt.chars().count() as u32;
            let output_count = content.chars().count() as u32;
            
            return Ok(AIResponse {
                content,
                explanation: Some("Resolved by OpenAI API (mock implementation)".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: token_count,
                    output_tokens: output_count,
                    total_tokens: token_count + output_count,
                }),
                model: self.config.model.clone(),
            });
        }
        
        // Create a request to the OpenAI API
        let api_endpoint = self.config.base_url.clone().unwrap_or_else(|| {
            "https://api.openai.com/v1/chat/completions".to_string()
        });
        
        // Create the request payload
        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ],
            "temperature": 0.2
        });
        
        // Add max_tokens if specified
        let max_tokens = self.config.additional_settings.get("max_tokens")
            .and_then(|s| s.parse::<u32>().ok());
        
        let payload = if let Some(max_tokens) = max_tokens {
            let mut payload_obj = payload.as_object().unwrap().clone();
            payload_obj.insert("max_tokens".to_string(), serde_json::Value::Number(serde_json::Number::from(max_tokens)));
            serde_json::Value::Object(payload_obj)
        } else {
            payload
        };
        
        // Prepare to send the request to OpenAI API
        debug!("Sending request to OpenAI API at {}", api_endpoint);
        
        // Create request agent
        let mut request = ureq::post(&api_endpoint)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.config.api_key));
            
        // Add organization ID if provided
        if let Some(org_id) = &self.config.org_id {
            request = request.set("OpenAI-Organization", org_id);
        }
        
        // Send the request to the OpenAI API
        // Send the request to the OpenAI API
        let response = request
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
                    ureq::Error::Transport(transport) => AIProviderError::ConnectionError(format!("Failed to connect to OpenAI API: {}", transport)),
                }
            })?;
            
        // Parse the response
        let response_json: serde_json::Value = response.into_json()
            .map_err(|e| AIProviderError::ResponseError(format!("Failed to parse response: {}", e)))?;
        
        // Extract the response content from the JSON
        let content = self.extract_response_content(&response_json)?;
        
        debug!("Received response from OpenAI API");
        let resolved_content = self.parse_response(&content)?;
        
        // Extract token usage from the response
        let token_usage = self.extract_token_usage(&response_json);
        
        // Extract explanation (finish reason) from the response
        let explanation = if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(finish_reason) = first_choice.get("finish_reason").and_then(|r| r.as_str()) {
                    Some(format!("OpenAI finished response with reason: {}", finish_reason))
                } else {
                    Some("Resolved by OpenAI API".to_string())
                }
            } else {
                Some("Resolved by OpenAI API".to_string())
            }
        } else {
            Some("Resolved by OpenAI API".to_string())
        };
        
        Ok(AIResponse {
            content: resolved_content,
            explanation,
            token_usage,
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
        
        let system_prompt = self.get_system_prompt();
        let user_prompt = self.prompt_generator.generate_file_prompt(conflict_file);
        
        info!("Sending entire file to OpenAI for resolution with model: {}", self.config.model);
        debug!("System prompt: {}", system_prompt);
        debug!("User prompt: {}", user_prompt);
        
        // If we're in test mode, return a mock response
        if cfg!(test) || env::var("TEST_MODE").unwrap_or_else(|_| "false".to_string()) == "true" {
            // For testing with the example merge conflicts file
            let is_merge_example = conflict_file.path.contains("merge_conflicts_example.sh");
            
            let content = if is_merge_example {
                // Return a complete resolution for the example file
                "#!/bin/bash\n\n# A script demonstrating complex merge conflicts\n\n# Database connection settings\nDB_HOST=\"replica.db.example.com\" # Using replica from feature/app-metrics\nDB_PORT=5432\nDB_USER=\"app_user\"\nDB_PASSWORD=\"new_very_secure_password\" # Using newer password from feature/app-metrics\nDB_NAME=\"production_db\"\n\n# Function to check dependencies\ncheck_dependencies() {\n    echo \"Checking dependencies...\"\n    for dep in \"curl\" \"jq\" \"wget\"; do\n        if ! command -v $dep &> /dev/null; then\n            install_dependency $dep\n        fi\n    done\n}\n\ninstall_dependency() {\n    echo \"Installing $1...\"\n    # Implementation details\n}\n\n# Function to handle errors\nhandle_error() {\n    echo \"Error: $1\"\n    exit 1\n}\n\n# Main application function\nmain() {\n    # Parse command line arguments\n    parse_arguments \"$@\"\n    \n    # Initialize the application\n    check_dependencies\n    setup_database_connection\n    setup_cache\n    initialize_metrics\n    \n    # Start application\n    echo \"Starting application with $(get_thread_count) threads...\"\n    start_worker_processes\n    setup_signal_handlers\n    wait_for_completion\n}\n\nparse_arguments() {\n    # Parse command line arguments\n    while [[ $# -gt 0 ]]; do\n        case $1 in\n            --debug) DEBUG_MODE=true ;;\n            --threads=*) THREAD_COUNT=\"${1#*=}\" ;;\n            *) echo \"Unknown option: $1\" ;;\n        esac\n        shift\n    done\n}\n\nget_thread_count() {\n    echo ${THREAD_COUNT:-$(nproc)}\n}\n\n# Call main function with arguments\nmain \"$@\"\n".to_string()
            } else {
                // Default mock response
                "// Mock resolved file content from OpenAI API\nfunction example() {\n    // Combined implementation\n    console.log('Resolved content');\n}\n\nfunction anotherFunction() {\n    // This function was also resolved\n    return true;\n}\n".to_string()
            };
            
            let token_count = user_prompt.chars().count() as u32;
            let output_count = content.chars().count() as u32;
            
            return Ok(AIResponse {
                content,
                explanation: Some("Entire file resolved by OpenAI API (mock implementation)".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: token_count,
                    output_tokens: output_count,
                    total_tokens: token_count + output_count,
                }),
                model: self.config.model.clone(),
            });
        }
        
        // Create a request to the OpenAI API
        let api_endpoint = self.config.base_url.clone().unwrap_or_else(|| {
            "https://api.openai.com/v1/chat/completions".to_string()
        });
        
        // Create the request payload
        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ],
            "temperature": 0.2
        });
        
        // Add max_tokens if specified
        let max_tokens = self.config.additional_settings.get("max_tokens")
            .and_then(|s| s.parse::<u32>().ok());
        
        let payload = if let Some(max_tokens) = max_tokens {
            let mut payload_obj = payload.as_object().unwrap().clone();
            payload_obj.insert("max_tokens".to_string(), serde_json::Value::Number(serde_json::Number::from(max_tokens)));
            serde_json::Value::Object(payload_obj)
        } else {
            payload
        };
        
        // Prepare to send the request to OpenAI API
        debug!("Sending request to OpenAI API at {}", api_endpoint);
        
        // Create request agent
        let mut request = ureq::post(&api_endpoint)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.config.api_key));
            
        // Add organization ID if provided
        if let Some(org_id) = &self.config.org_id {
            request = request.set("OpenAI-Organization", org_id);
        }
        
        let response = request
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
                    ureq::Error::Transport(transport) => AIProviderError::ConnectionError(format!("Failed to connect to OpenAI API: {}", transport)),
                }
            })?;
            
        // Parse the response
        let response_json: serde_json::Value = response.into_json()
            .map_err(|e| AIProviderError::ResponseError(format!("Failed to parse response: {}", e)))?;
        
        // Extract the response content from the JSON
        let content = self.extract_response_content(&response_json)?;
        
        debug!("Received response from OpenAI API");
        let resolved_content = self.parse_response(&content)?;
        
        // Extract token usage from the response
        let token_usage = self.extract_token_usage(&response_json);
        
        // Extract explanation (finish reason) from the response
        let explanation = if let Some(choices) = response_json.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(finish_reason) = first_choice.get("finish_reason").and_then(|r| r.as_str()) {
                    Some(format!("OpenAI finished response with reason: {}", finish_reason))
                } else {
                    Some("Resolved by OpenAI API".to_string())
                }
            } else {
                Some("Resolved by OpenAI API".to_string())
            }
        } else {
            Some("Resolved by OpenAI API".to_string())
        };
        
        Ok(AIResponse {
            content: resolved_content,
            explanation,
            token_usage,
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
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
        env::set_var("RIZZLER_OPENAI_MODEL", "gpt-4");
        env::set_var("RIZZLER_OPENAI_BASE_URL", "https://test-api.openai.com");
        env::set_var("RIZZLER_OPENAI_ORG_ID", "test-org");
        env::set_var("RIZZLER_SYSTEM_PROMPT", "Test system prompt");
        env::set_var("RIZZLER_TIMEOUT", "30");
        
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
        env::remove_var("RIZZLER_OPENAI_API_KEY");
        env::remove_var("RIZZLER_OPENAI_MODEL");
        env::remove_var("RIZZLER_OPENAI_BASE_URL");
        env::remove_var("RIZZLER_OPENAI_ORG_ID");
        env::remove_var("RIZZLER_SYSTEM_PROMPT");
        env::remove_var("RIZZLER_TIMEOUT");
    }
    
    #[test]
    fn test_get_system_prompt() {
        // Set the API key for testing
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
        
        // Test default system prompt
        {
            let provider = OpenAIProvider::new().unwrap();
            let system_prompt = provider.get_system_prompt();
            assert!(system_prompt.contains("Git merge conflicts") || 
                   system_prompt.contains("software developer") || 
                   system_prompt.contains("semantic"));
        }
        
        // Test custom system prompt
        {
            env::set_var("RIZZLER_SYSTEM_PROMPT", "Custom system prompt");
            let provider = OpenAIProvider::new().unwrap();
            let system_prompt = provider.get_system_prompt();
            assert_eq!(system_prompt, "Custom system prompt");
            env::remove_var("RIZZLER_SYSTEM_PROMPT");
        }
        
        // Clean up environment
        env::remove_var("RIZZLER_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_conflict_prompt_generation() {
        // Set the API key for testing
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = OpenAIProvider::new().unwrap();
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\nwith multiple lines\n", "Their content\nalso with lines\n");
        let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Generate conflict prompt using prompt generator
        let prompt = provider.prompt_generator.generate_conflict_prompt(&conflict_file, &conflict);
        
        // Check prompt content
        assert!(prompt.contains("Git merge conflict") || prompt.contains("CONFLICT DETAILS"));
        assert!(prompt.contains("OUR VERSION"));
        assert!(prompt.contains("THEIR VERSION"));
        assert!(prompt.contains("Our content"));
        assert!(prompt.contains("Their content"));
        
        // Clean up environment
        env::remove_var("RIZZLER_OPENAI_API_KEY");
    }
    
    #[test]
    fn test_resolve_conflict() {
        // Set the API key for testing and test mode
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
        env::set_var("TEST_MODE", "true"); // This ensures we don't make real API calls in tests
        
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
        
        // Clean up environment
        env::remove_var("RIZZLER_OPENAI_API_KEY");
        env::remove_var("TEST_MODE");
    }
    
    proptest! {
        #[test]
        fn test_conflict_prompt_generation_prop(our_content in r"[\w\s]{1,100}", their_content in r"[\w\s]{1,100}") {
            // Set the API key for testing
            env::set_var("RIZZLER_OPENAI_API_KEY", "test-api-key");
            
            // Create a provider
            let provider = OpenAIProvider::new().unwrap();
            
            // Create a test conflict
            let conflict = create_test_conflict(&our_content, &their_content);
            let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
            
            // Generate conflict prompt using prompt generator
            let prompt = provider.prompt_generator.generate_conflict_prompt(&conflict_file, &conflict);
            
            // Check that the prompt contains the content we provided
            prop_assert!(prompt.contains(&our_content));
            prop_assert!(prompt.contains(&their_content));
            
            // Clean up environment
            env::remove_var("RIZZLER_OPENAI_API_KEY");
        }
    }
}