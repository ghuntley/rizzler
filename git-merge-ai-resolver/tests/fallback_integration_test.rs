// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use git_merge_ai_resolver::ai_resolution::{AIResolutionStrategy, AIFileResolutionStrategy};
use git_merge_ai_resolver::fallback::FallbackResolutionStrategy;
use git_merge_ai_resolver::conflict_parser::{ConflictFile, ConflictRegion};
use git_merge_ai_resolver::resolution_engine::{ResolutionStrategy, ResolutionError};
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

// Test that AIResolutionStrategy with_fallback correctly uses the fallback chain
#[test]
fn test_ai_resolution_strategy_with_fallback() {
    // Set environment variables for testing
    env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
    env::set_var("GIT_MERGE_CLAUDE_API_KEY", "test-api-key");
    
    // Test creating an AIResolutionStrategy with fallback
    let strategy = AIResolutionStrategy::with_fallback("openai,claude");
    assert!(strategy.is_ok());
    
    let strategy = strategy.unwrap();
    
    // Create a test conflict
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    
    // Check if it can handle conflicts
    assert!(strategy.can_handle(&conflict));
    
    // Test resolving a conflict
    let result = strategy.resolve_conflict(&conflict);
    assert!(result.is_ok());
    
    // Clean up environment
    env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    env::remove_var("GIT_MERGE_CLAUDE_API_KEY");
}

// Test that AIFileResolutionStrategy with_fallback correctly uses the fallback chain
#[test]
fn test_ai_file_resolution_strategy_with_fallback() {
    // Set environment variables for testing
    env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
    env::set_var("GIT_MERGE_CLAUDE_API_KEY", "test-api-key");
    
    // Test creating an AIFileResolutionStrategy with fallback
    let strategy = AIFileResolutionStrategy::with_fallback("openai,claude");
    assert!(strategy.is_ok());
    
    let strategy = strategy.unwrap();
    
    // Create a test conflict file
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    let conflict_file = create_test_conflict_file(vec![conflict]);
    
    // Test resolving a file
    let result = strategy.resolve_file(&conflict_file);
    assert!(result.is_ok());
    
    // Clean up environment
    env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    env::remove_var("GIT_MERGE_CLAUDE_API_KEY");
}

// Test that AIResolutionStrategy with_fallback falls back to another provider when the first one fails
#[test]
fn test_ai_resolution_strategy_with_fallback_failover() {
    // Only set Claude API key to force fallback from OpenAI to Claude
    env::set_var("GIT_MERGE_CLAUDE_API_KEY", "test-api-key");
    
    // Test creating an AIResolutionStrategy with fallback
    let strategy = AIResolutionStrategy::with_fallback("openai,claude");
    assert!(strategy.is_ok());
    
    let strategy = strategy.unwrap();
    
    // Create a test conflict
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    
    // Test resolving a conflict
    let result = strategy.resolve_conflict(&conflict);
    assert!(result.is_ok());
    
    // Clean up environment
    env::remove_var("GIT_MERGE_CLAUDE_API_KEY");
}

// Test that AIFileResolutionStrategy with_fallback falls back to another provider when the first one fails
#[test]
fn test_ai_file_resolution_strategy_with_fallback_failover() {
    // Only set Claude API key to force fallback from OpenAI to Claude
    env::set_var("GIT_MERGE_CLAUDE_API_KEY", "test-api-key");
    
    // Test creating an AIFileResolutionStrategy with fallback
    let strategy = AIFileResolutionStrategy::with_fallback("openai,claude");
    assert!(strategy.is_ok());
    
    let strategy = strategy.unwrap();
    
    // Create a test conflict file
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    let conflict_file = create_test_conflict_file(vec![conflict]);
    
    // Test resolving a file
    let result = strategy.resolve_file(&conflict_file);
    assert!(result.is_ok());
    
    // Clean up environment
    env::remove_var("GIT_MERGE_CLAUDE_API_KEY");
}