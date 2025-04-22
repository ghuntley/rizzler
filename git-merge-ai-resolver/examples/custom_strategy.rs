// Example of implementing a custom resolution strategy

use git_merge_ai_resolver::{
    conflict_parser::{ConflictFile, ConflictRegion},
    resolution_engine::{ResolutionEngine, ResolutionError, ResolutionStrategy},
};
use std::fs;

// A custom strategy that always takes the longer version of a conflict
struct LongerVersionStrategy;

impl LongerVersionStrategy {
    fn new() -> Self {
        LongerVersionStrategy {}
    }
}

impl ResolutionStrategy for LongerVersionStrategy {
    fn name(&self) -> &str {
        "longer-version"
    }
    
    fn can_handle(&self, _conflict: &ConflictRegion) -> bool {
        // This strategy can handle any conflict
        true
    }
    
    fn resolve_conflict(&self, conflict: &ConflictRegion) -> Result<String, ResolutionError> {
        // Choose the longer version (or ours if they're the same length)
        if conflict.their_content.len() > conflict.our_content.len() {
            Ok(conflict.their_content.clone())
        } else {
            Ok(conflict.our_content.clone())
        }
    }
}

fn main() {
    // Check if a file path was provided
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <path-to-conflict-file>", args[0]);
        return;
    }
    
    let file_path = &args[1];
    
    // Read the file content
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            println!("Error reading file: {}", err);
            return;
        }
    };
    
    // Parse the file to find conflicts
    let parser = git_merge_ai_resolver::conflict_parser::parse_conflict_file_with_base;
    let conflict_file = match parser(&content, file_path) {
        Ok(file) => file,
        Err(err) => {
            println!("Error parsing conflicts: {}", err);
            return;
        }
    };
    
    println!("Found {} conflicts in file {}", conflict_file.conflicts.len(), file_path);
    
    // Create a resolution engine
    let mut engine = ResolutionEngine::new();
    
    // Add our custom strategy
    engine.add_strategy(Box::new(LongerVersionStrategy::new()));
    
    // Resolve conflicts
    match engine.resolve_with_strategy(&conflict_file, "longer-version") {
        Ok(resolution) => {
            println!("\nResolved conflicts using strategy: {}", resolution.strategy_name);
            println!("Resolved {}/{} conflicts", resolution.resolved_count, resolution.resolved_count + resolution.unresolved_count);
            
            // Write the resolved content to a new file
            let output_path = format!("{}.resolved", file_path);
            match fs::write(&output_path, &resolution.content) {
                Ok(_) => println!("Wrote resolved content to {}", output_path),
                Err(err) => println!("Error writing resolved content: {}", err),
            }
        },
        Err(err) => {
            println!("Error resolving conflicts: {}", err);
        }
    }
}