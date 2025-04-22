// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use rizzler::providers::GeminiProvider;
use rizzler::ai_provider::{AIProvider, AIProviderError};
use rizzler::conflict_parser::{ConflictFile, ConflictRegion};
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

#[cfg(feature = "integration-tests")]
#[test]
fn test_gemini_api_integration() {
    // Skip this test unless the Gemini API key is properly set
    let api_key = match env::var("RIZZLER_GEMINI_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            println!("Skipping test_gemini_api_integration: RIZZLER_GEMINI_API_KEY not set");
            return;
        }
    };
    
    // Create a provider with the real API key
    let provider = GeminiProvider::new().unwrap();
    
    // Create a simple test conflict for resolution
    let conflict = create_test_conflict(
        "function calculateSum(a, b) {\n  return a + b;\n}\n", 
        "function calculateSum(a, b) {\n  // Add two numbers and return the result\n  return a + b;\n}\n"
    );
    let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
    
    // Test the whole file resolution approach
    let file_result = provider.resolve_file(&conflict_file);
    assert!(file_result.is_ok(), "File resolution failed: {:?}", file_result.err());
    
    let file_response = file_result.unwrap();
    assert!(!file_response.content.is_empty(), "Empty response content");
    assert!(file_response.explanation.is_some(), "Missing explanation");
    assert!(file_response.token_usage.is_some(), "Missing token usage");
    
    // Test the specific conflict resolution approach
    let conflict_result = provider.resolve_conflict(&conflict_file, &conflict);
    assert!(conflict_result.is_ok(), "Conflict resolution failed: {:?}", conflict_result.err());
    
    let conflict_response = conflict_result.unwrap();
    assert!(!conflict_response.content.is_empty(), "Empty conflict resolution content");
    assert!(conflict_response.explanation.is_some(), "Missing conflict explanation");
    assert!(conflict_response.token_usage.is_some(), "Missing conflict token usage");
    
    // Verify the content is sensible (should contain function definition)
    assert!(conflict_response.content.contains("function calculateSum"), "Content missing expected function name");
    assert!(conflict_response.content.contains("return a + b"), "Content missing expected return statement");
}