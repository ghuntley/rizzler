// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use rizzler::ai_provider::{AIResponse, TokenUsage};
use rizzler::cache::AIResolutionCache;
use rizzler::conflict_parser::{ConflictRegion, ConflictFile};
use std::env;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

// Helper function to create a test conflict region
fn create_test_conflict(our_content: &str, their_content: &str) -> ConflictRegion {
    ConflictRegion {
        base_content: String::from("Base content\n"),
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

// Helper function to create a test response
fn create_test_response(content: &str) -> AIResponse {
    AIResponse {
        content: content.to_string(),
        model: "test-model".to_string(),
        explanation: Some("Test explanation".to_string()),
        token_usage: Some(TokenUsage {
            input_tokens: 5,
            output_tokens: 5,
            total_tokens: 10,
        }),
    }
}

fn test_disk_cache_store_retrieve() -> Result<(), String> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let cache_dir = temp_dir.path().to_path_buf();
    
    println!("Using temporary cache directory: {:?}", cache_dir);
    
    // Set the environment variable for the test
    env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // Create a cache with default settings
    let cache = AIResolutionCache::new();
    
    // Store and retrieve a conflict
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    let response = create_test_response("Resolved content\n");
    
    // Store in cache
    cache.put_conflict(&conflict, response.clone());
    
    // Verify cache file was created
    let conflicts_dir = cache_dir.join("conflicts");
    if !conflicts_dir.exists() {
        return Err("Conflicts directory was not created".to_string());
    }
    
    // Count the number of files in the conflicts directory
    let files: Vec<_> = fs::read_dir(&conflicts_dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    
    println!("Number of files in conflicts directory: {}", files.len());
    if files.len() != 1 {
        return Err(format!("Expected 1 file in conflicts directory, got {}", files.len()));
    }
    
    // Retrieve from cache
    let cached = cache.get_conflict(&conflict);
    if cached.is_none() {
        return Err("Failed to retrieve conflict from cache".to_string());
    }
    
    let cached = cached.unwrap();
    if cached.content != "Resolved content\n" {
        return Err(format!("Retrieved content doesn't match original: {}", cached.content));
    }
    
    // Clean up
    env::remove_var("RIZZLER_CACHE_DIR");
    
    Ok(())
}

fn test_disk_cache_file_operations() -> Result<(), String> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let cache_dir = temp_dir.path().to_path_buf();
    
    // Set the environment variable for the test
    env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // Create a cache with default settings
    let cache = AIResolutionCache::new();
    
    // Store and retrieve a file
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    let file = create_test_conflict_file(vec![conflict]);
    let response = create_test_response("Resolved file content\n");
    
    // Store in cache
    cache.put_file(&file, response.clone());
    
    // Verify cache file was created
    let files_dir = cache_dir.join("files");
    if !files_dir.exists() {
        return Err("Files directory was not created".to_string());
    }
    
    // Count the number of files in the files directory
    let files: Vec<_> = fs::read_dir(&files_dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    
    println!("Number of files in files directory: {}", files.len());
    if files.len() != 1 {
        return Err(format!("Expected 1 file in files directory, got {}", files.len()));
    }
    
    // Retrieve from cache
    let cached = cache.get_file(&file);
    if cached.is_none() {
        return Err("Failed to retrieve file from cache".to_string());
    }
    
    let cached = cached.unwrap();
    if cached.content != "Resolved file content\n" {
        return Err(format!("Retrieved content doesn't match original: {}", cached.content));
    }
    
    // Clean up
    env::remove_var("RIZZLER_CACHE_DIR");
    
    Ok(())
}

fn test_disk_cache_expiration() -> Result<(), String> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let cache_dir = temp_dir.path().to_path_buf();
    
    println!("Expiration test using cache directory: {:?}", cache_dir);
    
    // Set the environment variable for the test
    env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // Create a cache with a very short TTL
    let mut cache = AIResolutionCache::with_ttl(Duration::from_millis(500));
    cache.set_cache_dir(cache_dir.clone());
    
    // Create conflict and response
    let conflict = create_test_conflict("Expiring content\n", "Their content\n");
    let response = create_test_response("Expiring resolved content\n");
    
    println!("Putting item in cache with 500ms TTL");
    
    // Store in cache
    cache.put_conflict(&conflict, response);
    
    // Explicitly flush to ensure the entry is written to disk
    cache.flush().map_err(|e| e.to_string())?;
    
    println!("Flushed cache to disk");
    
    // Get the cache file path
    let conflicts_dir = cache_dir.join("conflicts");
    
    // Verify cache directory exists
    if !conflicts_dir.exists() {
        return Err("Cache directory was not created".to_string());
    }
    
    // List files in the cache directory
    let files: Vec<_> = fs::read_dir(&conflicts_dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    
    println!("Number of files in cache directory: {}", files.len());
    
    if files.len() != 1 {
        return Err(format!("Expected 1 file in conflicts directory, got {}", files.len()));
    }
    
    println!("Cache file exists at: {:?}", files[0].path());
    
    // Sleep to allow for expiration
    println!("Sleeping for 1000ms to allow cache entry to expire");
    std::thread::sleep(Duration::from_millis(1000));
    
    // Create a new cache pointing to the same directory
    // This will load from disk and should respect TTL
    let new_cache = AIResolutionCache::with_ttl(Duration::from_millis(500));
    
    // Try to get the expired entry
    match new_cache.get_conflict(&conflict) {
        Some(_) => return Err("Content should have expired".to_string()),
        None => println!("Cached item has expired as expected"),
    }
    
    // Clean up
    env::remove_var("RIZZLER_CACHE_DIR");
    
    Ok(())
}

fn test_disk_cache_persistence() -> Result<(), String> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let cache_dir = temp_dir.path().to_path_buf();
    
    // Set the environment variable for the test
    env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // First cache instance
    let cache1 = AIResolutionCache::new();
    
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    let response = create_test_response("Resolved content\n");
    
    // Store in first cache
    cache1.put_conflict(&conflict, response.clone());
    
    // Create a second cache instance pointing to the same directory
    let cache2 = AIResolutionCache::new();
    
    // Verify second cache can retrieve the item
    let cached = cache2.get_conflict(&conflict);
    if cached.is_none() {
        return Err("Failed to retrieve persisted conflict from new cache instance".to_string());
    }
    
    let cached = cached.unwrap();
    if cached.content != "Resolved content\n" {
        return Err(format!("Retrieved content from new cache instance doesn't match: {}", cached.content));
    }
    
    // Clean up
    env::remove_var("RIZZLER_CACHE_DIR");
    
    Ok(())
}

fn test_disk_cache_clear() -> Result<(), String> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let cache_dir = temp_dir.path().to_path_buf();
    
    // Set the environment variable for the test
    env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // Create a cache
    let cache = AIResolutionCache::new();
    
    // Add items to the cache
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    let file = create_test_conflict_file(vec![conflict.clone()]);
    let response1 = create_test_response("Resolved content\n");
    let response2 = create_test_response("Resolved file content\n");
    
    cache.put_conflict(&conflict, response1);
    cache.put_file(&file, response2);
    
    // Verify both are retrievable
    if cache.get_conflict(&conflict).is_none() {
        return Err("Failed to retrieve conflict before clearing".to_string());
    }
    if cache.get_file(&file).is_none() {
        return Err("Failed to retrieve file before clearing".to_string());
    }
    
    // Clear the cache
    cache.clear();
    
    // Verify nothing is retrievable after clearing
    if cache.get_conflict(&conflict).is_some() {
        return Err("Cache should be empty after clear (conflict still exists)".to_string());
    }
    if cache.get_file(&file).is_some() {
        return Err("Cache should be empty after clear (file still exists)".to_string());
    }
    
    // Clean up
    env::remove_var("RIZZLER_CACHE_DIR");
    
    Ok(())
}

fn test_disk_cache_max_entries() -> Result<(), String> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let cache_dir = temp_dir.path().to_path_buf();
    
    // Set the environment variable for the test
    env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // Create a cache with max entries set to 2
    let mut cache = AIResolutionCache::new();
    cache.set_max_entries(2);
    
    // Add 3 items to exceed the limit
    let conflict1 = create_test_conflict("Content 1\n", "Their content\n");
    let conflict2 = create_test_conflict("Content 2\n", "Their content\n");
    let conflict3 = create_test_conflict("Content 3\n", "Their content\n");
    
    let response1 = create_test_response("Resolved content 1\n");
    let response2 = create_test_response("Resolved content 2\n");
    let response3 = create_test_response("Resolved content 3\n");
    
    // Store all three items with delays to ensure different timestamps
    cache.put_conflict(&conflict1, response1);
    std::thread::sleep(Duration::from_millis(10));
    
    cache.put_conflict(&conflict2, response2);
    std::thread::sleep(Duration::from_millis(10));
    
    cache.put_conflict(&conflict3, response3);
    
    // Verify only the 2 most recent are retrievable
    if cache.get_conflict(&conflict1).is_some() {
        return Err("Oldest item should have been evicted".to_string());
    }
    if cache.get_conflict(&conflict2).is_none() {
        return Err("Second item should be retrievable".to_string());
    }
    if cache.get_conflict(&conflict3).is_none() {
        return Err("Most recent item should be retrievable".to_string());
    }
    
    // Clean up
    env::remove_var("RIZZLER_CACHE_DIR");
    
    Ok(())
}

fn main() {
    println!("Running disk cache tests...");
    
    // Create an array of test functions with explicit type annotations 
    let tests: [(&str, fn() -> Result<(), String>); 6] = [
        ("Store and retrieve", test_disk_cache_store_retrieve),
        ("File operations", test_disk_cache_file_operations),
        ("Expiration", test_disk_cache_expiration),
        ("Persistence", test_disk_cache_persistence),
        ("Clear", test_disk_cache_clear),
        ("Max entries", test_disk_cache_max_entries),
    ];
    
    let mut failures = 0;
    
    for (name, test) in tests.iter() {
        print!("Running test: {}... ", name);
        match test() {
            Ok(_) => println!("OK"),
            Err(e) => {
                println!("FAILED: {}", e);
                failures += 1;
            }
        }
    }
    
    println!("Tests completed: {} passed, {} failed", tests.len() - failures, failures);
    
    if failures > 0 {
        std::process::exit(1);
    }
} 