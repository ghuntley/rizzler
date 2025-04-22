// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use git_merge_ai_resolver::prompt_engineering::{PromptGenerator, PromptTemplate};
use git_merge_ai_resolver::conflict_parser::{ConflictFile, ConflictRegion};
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
fn test_default_prompt_template() {
    // Create a generator with the default template
    let generator = PromptGenerator::new(PromptTemplate::Default);
    
    // Create a test conflict
    let conflict = create_test_conflict("function add(a, b) {\n  return a + b;\n}\n", 
                                      "function add(a, b) {\n  // Add two numbers\n  return a + b;\n}\n");
    let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
    
    // Generate system and user prompts
    let system_prompt = generator.generate_system_prompt();
    let user_prompt = generator.generate_conflict_prompt(&conflict_file, &conflict);
    
    // Check that the prompts are not empty
    assert!(!system_prompt.is_empty(), "System prompt should not be empty");
    assert!(!user_prompt.is_empty(), "User prompt should not be empty");
    
    // Check that the prompts contain expected elements
    assert!(system_prompt.contains("resolve Git merge conflicts"), "System prompt should mention resolving Git merge conflicts");
    assert!(user_prompt.contains("OUR VERSION"), "User prompt should include OUR VERSION section");
    assert!(user_prompt.contains("THEIR VERSION"), "User prompt should include THEIR VERSION section");
    
    // Check that conflict content is included
    assert!(user_prompt.contains("function add"), "User prompt should include the function declaration");
    assert!(user_prompt.contains("return a + b"), "User prompt should include the return statement");
}

#[test]
fn test_enhanced_prompt_template() {
    // Create a generator with the enhanced template
    let generator = PromptGenerator::new(PromptTemplate::Enhanced);
    
    // Create a test conflict with function-like content
    let conflict = create_test_conflict(
        "function calculateSum(a, b) {\n  return a + b;\n}\n", 
        "function calculateSum(a, b) {\n  // Add two numbers\n  return a + b;\n}\n"
    );
    let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
    
    // Generate system and user prompts
    let system_prompt = generator.generate_system_prompt();
    let user_prompt = generator.generate_conflict_prompt(&conflict_file, &conflict);
    
    // Check that the prompts contain the enhanced elements
    assert!(system_prompt.contains("semantic understanding"), "Enhanced system prompt should mention semantic understanding");
    assert!(user_prompt.contains("CONFLICT ANALYSIS"), "Enhanced user prompt should include a CONFLICT ANALYSIS section");
}

#[test]
fn test_file_prompt_generation() {
    // Create a generator with the default template
    let generator = PromptGenerator::new(PromptTemplate::Default);
    
    // Create multiple test conflicts
    let conflict1 = create_test_conflict("function add(a, b) {\n  return a + b;\n}\n", 
                                       "function add(a, b) {\n  // Add two numbers\n  return a + b;\n}\n");
    let conflict2 = create_test_conflict("function subtract(a, b) {\n  return a - b;\n}\n", 
                                       "function subtract(a, b) {\n  // Subtract b from a\n  return a - b;\n}\n");
    let conflict_file = create_test_conflict_file(vec![conflict1, conflict2]);
    
    // Generate a prompt for the entire file
    let file_prompt = generator.generate_file_prompt(&conflict_file);
    
    // Check that the prompt includes information about both conflicts
    assert!(file_prompt.contains("add(a, b)"), "File prompt should include the first conflict");
    assert!(file_prompt.contains("subtract(a, b)"), "File prompt should include the second conflict");
    
    // Check that the prompt includes the number of conflicts
    assert!(file_prompt.contains("has 2 conflict"), "File prompt should mention the number of conflicts");
}

#[test]
fn test_custom_system_prompt() {
    // Set a custom system prompt in the environment
    env::set_var("GIT_MERGE_AI_SYSTEM_PROMPT", "Custom system prompt for testing");
    
    // Create a generator with the default template
    let generator = PromptGenerator::new(PromptTemplate::Default);
    
    // Generate a system prompt
    let system_prompt = generator.generate_system_prompt();
    
    // Check that the custom prompt is used
    assert_eq!(system_prompt, "Custom system prompt for testing");
    
    // Clean up environment
    env::remove_var("GIT_MERGE_AI_SYSTEM_PROMPT");
}

#[test]
fn test_prompt_with_context() {
    // Create a generator with the context-aware template
    let generator = PromptGenerator::new(PromptTemplate::ContextAware);
    
    // Create a test conflict with base content
    let mut conflict = create_test_conflict(
        "function calculateSum(a, b) {\n  return a + b;\n}\n", 
        "function calculateSum(a, b) {\n  // Add two numbers\n  return a + b;\n}\n"
    );
    conflict.base_content = "function calculateSum(a, b) {\n  // Original function\n  return a + b;\n}\n".to_string();
    
    let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
    
    // Generate a prompt
    let user_prompt = generator.generate_conflict_prompt(&conflict_file, &conflict);
    
    // Check that the prompt includes the base content
    assert!(user_prompt.contains("BASE VERSION"), "Context-aware prompt should include BASE VERSION section");
    assert!(user_prompt.contains("Original function"), "Context-aware prompt should include base content");
}