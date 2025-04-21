// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use git_merge_ai_resolver::GeminiProvider;
use git_merge_ai_resolver::ai_provider::{AIProvider, AIProviderError};
use git_merge_ai_resolver::conflict_parser::{ConflictFile, ConflictRegion};
use std::env;

#[ignore = "This test requires a valid Gemini API key"]
#[test]
fn test_gemini_api_integration() {
    // Skip test if no API key is provided
    let api_key = match env::var("GIT_MERGE_GEMINI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping test as GIT_MERGE_GEMINI_API_KEY is not set");
            return;
        }
    };
    
    // Create a test conflict
    let conflict = ConflictRegion {
        base_content: String::new(),
        our_content: "function add(a, b) {\n  return a + b;\n}\n".to_string(),
        their_content: "function add(a, b) {\n  // Add two numbers\n  return a + b;\n}\n".to_string(),
        start_line: 1,
        end_line: 5,
    };
    
    let conflict_file = ConflictFile {
        path: "test.js".to_string(),
        conflicts: vec![conflict.clone()],
        content: "<<<<<<< HEAD\nfunction add(a, b) {\n  return a + b;\n}\n=======\nfunction add(a, b) {\n  // Add two numbers\n  return a + b;\n}\n>>>>>>> feature-branch\n".to_string(),
    };
    
    // Create the provider
    let provider = GeminiProvider::new().unwrap();
    
    // Verify the provider is available
    assert!(provider.is_available());
    
    // Test resolving a conflict
    let result = provider.resolve_conflict(&conflict_file, &conflict);
    assert!(result.is_ok(), "Failed to resolve conflict: {:?}", result.err());
    
    let response = result.unwrap();
    println!("Resolved content: {}", response.content);
    println!("Explanation: {:?}", response.explanation);
    println!("Token usage: {:?}", response.token_usage);
    
    // Verify we got a meaningful response
    assert!(!response.content.is_empty());
    assert!(response.content.contains("function add"));
}