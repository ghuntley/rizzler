// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use rizzler::ai_provider::{AIProvider, AIProviderError, AIResponse};
use rizzler::conflict_parser::{ConflictFile, ConflictRegion};
use rizzler::providers::claude::ClaudeProvider;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

#[test]
fn test_claude_provider_integration() {
    // Skip this test if integration tests are not enabled
    if env::var("RIZZLER_RUN_INTEGRATION_TESTS").is_err() {
        return;
    }
    
    // Ensure we have an API key for testing
    let api_key = match env::var("RIZZLER_CLAUDE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping Claude integration test - no API key");
            return;
        }
    };
    
    if api_key.is_empty() {
        println!("Skipping Claude integration test - empty API key");
        return;
    }
    
    // Create a backup of the test file
    let file_path = "examples/merge_conflicts_example.sh";
    let backup_path = format!("{}.bak", file_path);
    
    let mut file_content = String::new();
    File::open(file_path)
        .and_then(|mut file| file.read_to_string(&mut file_content))
        .expect("Failed to read test file");
    
    // Write the backup
    File::create(&backup_path)
        .and_then(|mut file| file.write_all(file_content.as_bytes()))
        .expect("Failed to create backup file");
    
    // Create provider
    let provider = ClaudeProvider::new().expect("Failed to create Claude provider");
    
    // Test conflict parsing and resolution
    let content = file_content.clone();
    
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
    
    // Resolve conflicts
    for conflict in &conflicts {
        let result = provider.resolve_conflict(&conflict_file, conflict);
        match result {
            Ok(response) => {
                println!("Successfully resolved conflict at line {}: token usage: {:?}", 
                    conflict.start_line, response.token_usage);
                
                // Check that the resolved content doesn't contain conflict markers
                assert!(!response.content.contains("<<<<<<< HEAD"));
                assert!(!response.content.contains("======="));
                assert!(!response.content.contains(">>>>>>>"));
            },
            Err(e) => panic!("Failed to resolve conflict: {:?}", e),
        }
    }
    
    // Restore the backup file
    if Path::new(&backup_path).exists() {
        fs::copy(&backup_path, file_path).expect("Failed to restore backup file");
        fs::remove_file(&backup_path).expect("Failed to remove backup file");
    }
}