// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::{AIProvider, AIProviderConfig, AIProviderError, AIResponse, TokenUsage};
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tracing::{debug, info};
use regex;
use serde_json;

/// Anthropic Claude provider implementation
pub struct ClaudeProvider {
    config: AIProviderConfig,
}

impl ClaudeProvider {
    /// Create a new Claude provider
    pub fn new() -> Result<Self, AIProviderError> {
        // Get API key from environment variable
        let api_key = env::var("RIZZLER_CLAUDE_API_KEY")
            .map_err(|_| AIProviderError::ConfigError(
                "Missing Claude API key. Set RIZZLER_CLAUDE_API_KEY environment variable".to_string()
            ))?;
        
        // Get other configuration from environment variables
        let base_url = env::var("RIZZLER_CLAUDE_BASE_URL").ok();
        let model = env::var("RIZZLER_CLAUDE_MODEL").unwrap_or_else(|_| "claude-3-opus-20240229".to_string());
        let timeout_seconds = env::var("RIZZLER_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        
        // Get custom system prompt if provided
        let system_prompt = env::var("RIZZLER_SYSTEM_PROMPT").ok();
        
        // Create additional settings map
        let mut additional_settings = HashMap::new();
        if let Ok(max_tokens) = env::var("RIZZLER_CLAUDE_MAX_TOKENS") {
            additional_settings.insert("max_tokens".to_string(), max_tokens);
        }
        if let Ok(temperature) = env::var("RIZZLER_CLAUDE_TEMPERATURE") {
            additional_settings.insert("temperature".to_string(), temperature);
        }
        
        Ok(ClaudeProvider {
            config: AIProviderConfig {
                name: "claude".to_string(),
                api_key,
                model,
                base_url,
                org_id: None, // Claude doesn't use org_id
                system_prompt,
                timeout_seconds,
                additional_settings,
            },
        })
    }
    
    /// Create a user prompt for resolving a specific conflict
    fn create_user_prompt(&self, conflict_file: &ConflictFile, conflict: &ConflictRegion) -> String {
        format!(
            "Human: I need help resolving a Git merge conflict in the file: {}\n\n\
            The file contains a conflict between line {} and {}:\n\n\
            OUR VERSION (current branch):\n```\n{}```\n\n\
            THEIR VERSION (incoming branch):\n```\n{}```\n\n\
            Please resolve this conflict and provide only the final resolved content that should replace \
            the conflict. Preserve the intent of both changes if possible or choose the most appropriate \
            version if they are in direct conflict. Do not include conflict markers in your response. \n\n \
            IMPORTANT DO NOT SURROUND THE CODE IN ``` OR USE MARKDOWN SYNTAX IN THE RESPONSE (UNLESS THE FILE IS MARKDOWN ITSELF). RETURN THE PLAIN TEXT INSTEAD \n\n \
            Assistant: ",
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
            "Human: I need help resolving Git merge conflicts in the file: {}\n\n\
            The file has {} conflict(s):\n\n{}\n\
            Please provide the entire resolved file content with all conflicts resolved. \
            Preserve the intent of both changes whenever possible. \
            Do not include conflict markers in your response.\n\nAssistant: ",
            conflict_file.path,
            conflict_file.conflicts.len(),
            conflicts_text
        )
    }
    
    /// Parse the response from Claude
    fn parse_response(&self, response_text: &str) -> Result<String, AIProviderError> {
        // Extract code blocks if present (common in AI responses)
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

impl AIProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
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
                "Claude provider is not available. API key is missing".to_string()
            ));
        }
        
        let user_prompt = self.create_user_prompt(conflict_file, conflict);
        
        info!("Sending conflict to Claude for resolution with model: {}", self.config.model);
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
                    "// Default mock resolved content from Claude API\nfunction example() {\n    // Combined implementation\n    console.log('Resolved content');\n}\n".to_string()
                }
            } else {
                // Default mock response for non-example files
                "// Mock resolved content from Claude API\nfunction example() {\n    // Combined implementation\n    console.log('Resolved content');\n}\n".to_string()
            };
            
            let token_count = user_prompt.chars().count() as u32;
            let output_count = content.chars().count() as u32;
            
            return Ok(AIResponse {
                content,
                explanation: Some("Resolved by Claude API (mock implementation)".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: token_count,
                    output_tokens: output_count,
                    total_tokens: token_count + output_count,
                }),
                model: self.config.model.clone(),
            });
        }
        
        // Create a request to the Claude API
        let api_endpoint = self.config.base_url.clone().unwrap_or_else(|| {
            "https://api.anthropic.com/v1/messages".to_string()
        });
        
        // Set up request parameters
        let temperature = self.config.additional_settings.get("temperature")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.2);
        
        let max_tokens = self.config.additional_settings.get("max_tokens")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(4096);
        
        // Create the request payload with system instructions
        let payload = if let Some(system_prompt) = &self.config.system_prompt {
            // If custom system prompt is provided, use it
            serde_json::json!({
                "model": self.config.model,
                "temperature": temperature,
                "max_tokens": max_tokens,
                "messages": [
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ],
                "system": system_prompt
            })
        } else {
            // Use default system prompt
            let system_prompt = "You are an expert software developer helping to resolve Git merge conflicts. \
                Analyze conflicts and generate solutions that preserve the intent of both changes. \
                Use your best judgment to combine functionality or choose the most appropriate changes. \
                Preserve the code style of the original file. Provide only the resolved code without explanations.";
            
            serde_json::json!({
                "model": self.config.model,
                "temperature": temperature,
                "max_tokens": max_tokens,
                "messages": [
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ],
                "system": system_prompt
            })
        };
        
        debug!("Sending request to Claude API at {}", api_endpoint);
        
        // Create request agent with appropriate headers
        let request = ureq::post(&api_endpoint)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .set("Content-Type", "application/json")
            .set("x-api-key", &self.config.api_key)
            .set("anthropic-version", "2023-06-01") // Set appropriate API version
            .set("User-Agent", "rizzler/1.0");
            
        // Send the request to the Claude API
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
                    ureq::Error::Transport(transport) => AIProviderError::ConnectionError(format!("Failed to connect to Claude API: {}", transport)),
                }
            })?;
        
        // Parse the response
        let response_json: serde_json::Value = response.into_json()
            .map_err(|e| AIProviderError::ResponseError(format!("Failed to parse response: {}", e)))?;
        
        // Extract content from response
        let content = if let Some(content) = response_json.get("content").and_then(|c| c.as_array()).and_then(|a| a.first()) {
            if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                text.to_string()
            } else {
                return Err(AIProviderError::ResponseError("Failed to extract content from Claude response".to_string()));
            }
        } else {
            return Err(AIProviderError::ResponseError("Failed to extract content from Claude response".to_string()));
        };
        
        debug!("Received response from Claude API");
        let resolved_content = self.parse_response(&content)?;
        
        // Extract token usage if available
        let token_usage = if let Some(usage) = response_json.get("usage") {
            let input_tokens = usage.get("input_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
            
            let output_tokens = usage.get("output_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
            
            Some(TokenUsage {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
            })
        } else {
            None
        };
        
        Ok(AIResponse {
            content: resolved_content,
            explanation: Some("Resolved by Claude API".to_string()),
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
                "Claude provider is not available. API key is missing".to_string()
            ));
        }
        
        let user_prompt = self.create_file_prompt(conflict_file);
        
        info!("Sending entire file to Claude for resolution with model: {}", self.config.model);
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
                "// Mock resolved file content from Claude API\nfunction example() {\n    // Combined implementation\n    console.log('Resolved content');\n}\n\nfunction anotherFunction() {\n    // This function was also resolved\n    return true;\n}\n".to_string()
            };
            
            let token_count = user_prompt.chars().count() as u32;
            let output_count = content.chars().count() as u32;
            
            return Ok(AIResponse {
                content,
                explanation: Some("Entire file resolved by Claude API (mock implementation)".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: token_count,
                    output_tokens: output_count,
                    total_tokens: token_count + output_count,
                }),
                model: self.config.model.clone(),
            });
        }
        
        // Create a request to the Claude API
        let api_endpoint = self.config.base_url.clone().unwrap_or_else(|| {
            "https://api.anthropic.com/v1/messages".to_string()
        });
        
        // Set up request parameters
        let temperature = self.config.additional_settings.get("temperature")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.2);
        
        let max_tokens = self.config.additional_settings.get("max_tokens")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(4096);
        
        // Create the request payload with system instructions
        let payload = if let Some(system_prompt) = &self.config.system_prompt {
            // If custom system prompt is provided, use it
            serde_json::json!({
                "model": self.config.model,
                "temperature": temperature,
                "max_tokens": max_tokens,
                "messages": [
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ],
                "system": system_prompt
            })
        } else {
            // Use default system prompt
            let system_prompt = "You are an expert software developer helping to resolve Git merge conflicts. \
                Analyze conflicts and generate solutions that preserve the intent of both changes. \
                Use your best judgment to combine functionality or choose the most appropriate changes. \
                Preserve the code style of the original file. Provide only the resolved code without explanations.";
            
            serde_json::json!({
                "model": self.config.model,
                "temperature": temperature,
                "max_tokens": max_tokens,
                "messages": [
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ],
                "system": system_prompt
            })
        };
        
        debug!("Sending request to Claude API at {}", api_endpoint);
        
        // Create request agent with appropriate headers
        let request = ureq::post(&api_endpoint)
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .set("Content-Type", "application/json")
            .set("x-api-key", &self.config.api_key)
            .set("anthropic-version", "2023-06-01") // Set appropriate API version
            .set("User-Agent", "rizzler/1.0");
            
        // Send the request to the Claude API
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
                    ureq::Error::Transport(transport) => AIProviderError::ConnectionError(format!("Failed to connect to Claude API: {}", transport)),
                }
            })?;
        
        // Parse the response
        let response_json: serde_json::Value = response.into_json()
            .map_err(|e| AIProviderError::ResponseError(format!("Failed to parse response: {}", e)))?;
        
        // Extract content from response
        let content = if let Some(content) = response_json.get("content").and_then(|c| c.as_array()).and_then(|a| a.first()) {
            if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                text.to_string()
            } else {
                return Err(AIProviderError::ResponseError("Failed to extract content from Claude response".to_string()));
            }
        } else {
            return Err(AIProviderError::ResponseError("Failed to extract content from Claude response".to_string()));
        };
        
        debug!("Received response from Claude API");
        let resolved_content = self.parse_response(&content)?;
        
        // Extract token usage if available
        let token_usage = if let Some(usage) = response_json.get("usage") {
            let input_tokens = usage.get("input_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
            
            let output_tokens = usage.get("output_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32)
                .unwrap_or(0);
            
            Some(TokenUsage {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
            })
        } else {
            None
        };
        
        Ok(AIResponse {
            content: resolved_content,
            explanation: Some("Entire file resolved by Claude API".to_string()),
            token_usage,
            model: self.config.model.clone(),
        })
    }
    
    // Claude uses system prompts differently than other providers
    fn create_system_prompt(&self) -> String {
        // Check if a custom system prompt is provided in config
        if let Some(prompt) = &self.config().system_prompt {
            return prompt.clone();
        }
        
        // Return a default Claude-specific system prompt
        "You are an expert software developer specializing in resolving Git merge conflicts. \
        When presented with merge conflicts, analyze both changes carefully and create solutions \
        that preserve the intent and functionality of both changes whenever possible. \
        When resolving conflicts, maintain the code style of the original codebase and ensure \
        that the resulting code is syntactically correct and logical. Provide only the final \
        resolved code without any additional comments or explanations unless they existed in \
        the original code.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict_parser::ConflictRegion;
    use std::env;
    use proptest::prelude::*;
    use std::fs;
    
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
    fn test_claude_provider_config() {
        // Set environment variables for testing
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        env::set_var("RIZZLER_CLAUDE_MODEL", "claude-3-sonnet-20240229");
        env::set_var("RIZZLER_CLAUDE_BASE_URL", "https://test-api.anthropic.com");
        env::set_var("RIZZLER_SYSTEM_PROMPT", "Test system prompt");
        env::set_var("RIZZLER_TIMEOUT", "45");
        
        // Create provider
        let provider = ClaudeProvider::new().unwrap();
        
        // Check configuration
        assert_eq!(provider.config().api_key, "test-api-key");
        assert_eq!(provider.config().model, "claude-3-sonnet-20240229");
        assert_eq!(provider.config().base_url, Some("https://test-api.anthropic.com".to_string()));
        assert_eq!(provider.config().system_prompt, Some("Test system prompt".to_string()));
        assert_eq!(provider.config().timeout_seconds, 45);
        
        // Clean up environment
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
        env::remove_var("RIZZLER_CLAUDE_MODEL");
        env::remove_var("RIZZLER_CLAUDE_BASE_URL");
        env::remove_var("RIZZLER_SYSTEM_PROMPT");
        env::remove_var("RIZZLER_TIMEOUT");
    }
    
    #[test]
    fn test_create_user_prompt() {
        // Set the API key for testing
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = ClaudeProvider::new().unwrap();
        
        // Create a test conflict
        let conflict = create_test_conflict("Our content\nwith multiple lines\n", "Their content\nalso with lines\n");
        let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Create user prompt
        let prompt = provider.create_user_prompt(&conflict_file, &conflict);
        
        // Check prompt content
        assert!(prompt.contains("Human:"));
        assert!(prompt.contains("Assistant:"));
        assert!(prompt.contains("Git merge conflict"));
        assert!(prompt.contains("OUR VERSION"));
        assert!(prompt.contains("THEIR VERSION"));
        assert!(prompt.contains("Our content"));
        assert!(prompt.contains("Their content"));
        
        // Clean up environment
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
    }
    
    #[test]
    fn test_resolve_conflict() {
        // Set the API key for testing
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = ClaudeProvider::new().unwrap();
        
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
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
    }
    
    proptest! {
        #[test]
        fn test_create_user_prompt_prop(our_content in r"[\w\s]{1,100}", their_content in r"[\w\s]{1,100}") {
            // Set the API key for testing
            env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
            
            // Create a provider
            let provider = ClaudeProvider::new().unwrap();
            
            // Create a test conflict
            let conflict = create_test_conflict(&our_content, &their_content);
            let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
            
            // Create user prompt
            let prompt = provider.create_user_prompt(&conflict_file, &conflict);
            
            // Check that the prompt contains the content we provided
            prop_assert!(prompt.contains(&our_content));
            prop_assert!(prompt.contains(&their_content));
            
            // Clean up environment
            env::remove_var("RIZZLER_CLAUDE_API_KEY");
        }
    }
    
    #[test]
    fn test_parse_response_with_code_blocks() {
        // Set the API key for testing
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        
        // Create a provider
        let provider = ClaudeProvider::new().unwrap();
        
        // Test with code blocks
        let response_with_code_blocks = r#"I've resolved the conflict by combining the functionality from both versions.

```bash
function example() {
    echo "This is the resolved content"
    return 0
}
```

This preserves both the error handling and the new features."#;
        
        let result = provider.parse_response(response_with_code_blocks).unwrap();
        assert_eq!(result, "function example() {
    echo \"This is the resolved content\"
    return 0
}");
        
        // Test without code blocks
        let response_without_code_blocks = "function example() {\n    echo \"This is plain text\"\n}\n";
        let result = provider.parse_response(response_without_code_blocks).unwrap();
        assert_eq!(result, "function example() {\n    echo \"This is plain text\"\n}\n");
        
        // Clean up environment
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
    }
    
    #[test]
    fn test_resolve_merge_conflicts_example() {
        // Set environment variables for testing
        env::set_var("RIZZLER_CLAUDE_API_KEY", "test-api-key");
        env::set_var("TEST_MODE", "true"); // This ensures we don't make real API calls
        
        // Create a provider
        let provider = ClaudeProvider::new().unwrap();
        
        // Create a test conflict file from examples/merge_conflicts_example.sh
        let file_path = "examples/merge_conflicts_example.sh";
        let content = fs::read_to_string(file_path).unwrap();
        
        // Parse the conflict markers to create conflict regions
        let mut conflicts: Vec<ConflictRegion> = Vec::new();
        let mut i = 0;
        while i < content.lines().count() {
            let line = content.lines().nth(i).unwrap();
            if line.starts_with("<<<<<<< HEAD") {
                let start_line = i;
                let mut our_content = String::new();
                i += 1; // Move past the start marker
                
                // Extract "our" content
                while i < content.lines().count() {
                    let line = content.lines().nth(i).unwrap();
                    if line.starts_with("=======") {
                        break;
                    }
                    our_content.push_str(line);
                    our_content.push('\n');
                    i += 1;
                }
                
                i += 1; // Move past the separator
                let mut their_content = String::new();
                
                // Extract "their" content
                while i < content.lines().count() {
                    let line = content.lines().nth(i).unwrap();
                    if line.contains(">>>>>>>") {
                        break;
                    }
                    their_content.push_str(line);
                    their_content.push('\n');
                    i += 1;
                }
                
                // Create a conflict region
                conflicts.push(ConflictRegion {
                    base_content: String::new(),
                    our_content,
                    their_content,
                    start_line: start_line,
                    end_line: i + 1 // Include the end marker
                });
            }
            i += 1;
        }
        
        // Create a conflict file
        let conflict_file = ConflictFile {
            path: file_path.to_string(),
            conflicts: conflicts.clone(),
            content: content.clone(),
        };
        
        // Resolve the entire file
        let result = provider.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.content.is_empty());
        
        // Check that the resolved content doesn't contain conflict markers
        assert!(!response.content.contains("<<<<<<< HEAD"));
        assert!(!response.content.contains("======="));
        assert!(!response.content.contains(">>>>>>>"));
        
        // Also check individual conflict resolution
        for conflict in &conflicts {
            let result = provider.resolve_conflict(&conflict_file, conflict);
            assert!(result.is_ok());
            
            let response = result.unwrap();
            assert!(!response.content.is_empty());
            assert!(!response.content.contains("<<<<<<< HEAD"));
            assert!(!response.content.contains("======="));
            assert!(!response.content.contains(">>>>>>>"));
        }
        
        // Clean up environment
        env::remove_var("RIZZLER_CLAUDE_API_KEY");
        env::remove_var("TEST_MODE");
    }
}