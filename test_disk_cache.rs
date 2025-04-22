use rizzler::ai_provider::{AIResponse, TokenUsage};
use rizzler::cache::AIResolutionCache;
use rizzler::conflict_parser::{ConflictRegion, ConflictFile};
use std::env;
use std::fs;
use std::path::PathBuf;
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

fn main() {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    println!("Using temporary cache directory: {:?}", cache_dir);
    
    // Set the environment variable for the test
    env::set_var("RIZZLER_CACHE_DIR", cache_dir.to_str().unwrap());
    
    // Create a cache with default settings
    let cache = AIResolutionCache::new();
    
    // Test 1: Store and retrieve a conflict
    println!("Test 1: Store and retrieve a conflict");
    let conflict = create_test_conflict("Our content\n", "Their content\n");
    let response = create_test_response("Resolved content 1\n");
    
    // Store in cache
    cache.put_conflict(&conflict, response.clone());
    
    // Verify cache file was created
    let conflicts_dir = cache_dir.join("conflicts");
    assert!(conflicts_dir.exists(), "Conflicts directory was not created");
    
    // Count the number of files in the conflicts directory
    let files: Vec<_> = fs::read_dir(&conflicts_dir).unwrap()
        .filter_map(Result::ok)
        .collect();
    
    println!("Number of files in conflicts directory: {}", files.len());
    assert_eq!(files.len(), 1, "Expected 1 file in conflicts directory");
    
    // Retrieve from cache
    let cached = cache.get_conflict(&conflict);
    assert!(cached.is_some(), "Failed to retrieve conflict from cache");
    
    let cached = cached.unwrap();
    assert_eq!(cached.content, "Resolved content 1\n", "Retrieved content doesn't match original");
    
    // Test 2: Store and retrieve a file
    println!("Test 2: Store and retrieve a file");
    let file = create_test_conflict_file(vec![conflict.clone()]);
    let file_response = create_test_response("Resolved file content\n");
    
    // Store in cache
    cache.put_file(&file, file_response.clone());
    
    // Verify cache file was created
    let files_dir = cache_dir.join("files");
    assert!(files_dir.exists(), "Files directory was not created");
    
    // Count the number of files in the files directory
    let files: Vec<_> = fs::read_dir(&files_dir).unwrap()
        .filter_map(Result::ok)
        .collect();
    
    println!("Number of files in files directory: {}", files.len());
    assert_eq!(files.len(), 1, "Expected 1 file in files directory");
    
    // Retrieve from cache
    let cached = cache.get_file(&file);
    assert!(cached.is_some(), "Failed to retrieve file from cache");
    
    let cached = cached.unwrap();
    assert_eq!(cached.content, "Resolved file content\n", "Retrieved content doesn't match original");
    
    // Test 3: Test expiration
    println!("Test 3: Test expiration");
    
    // Create a new cache with a very short TTL
    let mut cache = AIResolutionCache::with_ttl(Duration::from_millis(50));
    cache.cache_dir = cache_dir.clone();
    
    let conflict2 = create_test_conflict("Expiring content\n", "Their content\n");
    let response2 = create_test_response("Expiring resolved content\n");
    
    // Store in cache
    cache.put_conflict(&conflict2, response2);
    
    // Verify it's initially retrievable
    assert!(cache.get_conflict(&conflict2).is_some(), "Failed to retrieve just-stored content");
    
    // Sleep to allow for expiration
    std::thread::sleep(Duration::from_millis(100));
    
    // Verify it's expired
    assert!(cache.get_conflict(&conflict2).is_none(), "Content should have expired");
    
    // Test 4: Test persistence (create a new cache instance)
    println!("Test 4: Test persistence");
    
    // Create a new cache pointing to the same directory
    let new_cache = AIResolutionCache::new();
    
    // Conflict from test 1 should still be retrievable 
    let cached = new_cache.get_conflict(&conflict);
    assert!(cached.is_some(), "Failed to retrieve persisted conflict from new cache instance");
    
    let cached = cached.unwrap();
    assert_eq!(cached.content, "Resolved content 1\n", "Retrieved content from new cache instance doesn't match");
    
    // Test 5: Test clear
    println!("Test 5: Test clear");
    
    // Clear the cache
    new_cache.clear();
    
    // Verify nothing is retrievable
    assert!(new_cache.get_conflict(&conflict).is_none(), "Cache should be empty after clear");
    assert!(new_cache.get_file(&file).is_none(), "Cache should be empty after clear");
    
    // Done
    println!("All tests passed!");
    
    // Clean up
    env::remove_var("RIZZLER_CACHE_DIR");
} 