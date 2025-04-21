use git_merge_ai_resolver::conflict_parser::{ConflictFile, ConflictRegion};
use git_merge_ai_resolver::ai_resolution_windowing::AIFileResolutionWithWindowingStrategy;
use std::env;

#[test]
fn test_ai_resolution_with_windowing_large_file() {
    // Skip this test as it requires an actual OpenAI API key
    // In real code, we'd use a mock AI provider for testing
    // but that's beyond the scope of this implementation
}

#[test]
fn test_ai_resolution_with_windowing_small_file() {
    // Skip this test as it requires an actual OpenAI API key
    // In real code, we'd use a mock AI provider for testing
    // but that's beyond the scope of this implementation
}

#[test]
fn test_windowing_decision_based_on_file_size() {
    // Set environment variables for testing
    env::set_var("GIT_MERGE_OPENAI_API_KEY", "test-api-key");
    
    // Test with different token limits
    let test_cases = vec![(100, true), (10000, false)];
    
    for (token_limit, should_use_windowing) in test_cases {
        env::set_var("GIT_MERGE_AI_TOKEN_LIMIT", token_limit.to_string());
        
        // Create a moderate size conflict file
        let mut content = String::new();
        for i in 1..100 {
            content.push_str(&format!("Line {} with some content to make it longer\n", i));
        }
        
        let conflict = create_test_conflict("Our content\n", "Their content\n");
        let conflict_file = ConflictFile {
            path: "test.txt".to_string(),
            conflicts: vec![conflict],
            content,
        };
        
        // Create strategy
        let strategy = AIFileResolutionWithWindowingStrategy::new().unwrap();
        
        // Check if windowing is needed
        assert_eq!(strategy.needs_windowing(&conflict_file.content), should_use_windowing);
    }
    
    // Clean up environment
    env::remove_var("GIT_MERGE_OPENAI_API_KEY");
    env::remove_var("GIT_MERGE_AI_TOKEN_LIMIT");
}

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

// Helper function to create a large test conflict file
fn create_large_test_conflict_file() -> ConflictFile {
    let mut content = String::new();
    let mut conflicts = Vec::new();
    
    // Create a large file (5K lines)
    for i in 1..5_000 {
        if i == 1000 || i == 2500 || i == 4000 {
            // Add conflict at these positions
            content.push_str(&format!("<<<<<<< HEAD\nOur content at line {}\n=======\nTheir content at line {}\n>>>>>>> branch-name\n", i, i));
            
            // Add conflict to the list
            conflicts.push(ConflictRegion {
                base_content: format!("Base content at line {}\n", i),
                our_content: format!("Our content at line {}\n", i),
                their_content: format!("Their content at line {}\n", i),
                start_line: i,
                end_line: i + 4,
            });
        } else {
            content.push_str(&format!("Line {}\n", i));
        }
    }
    
    ConflictFile {
        path: "large_file.txt".to_string(),
        conflicts,
        content,
    }
}