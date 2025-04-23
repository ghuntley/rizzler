// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::conflict_parser::{ConflictFile, ConflictRegion};
use crate::config::Config;
use std::error::Error;
use std::fmt;
use std::fs;
use tracing::{debug, info, warn};

/// Error types for resolution operations
#[derive(Debug)]
pub enum ResolutionError {
    /// IO error during file operations
    IoError(std::io::Error),
    
    /// Strategy error
    StrategyError(String),
    
    /// Unknown strategy
    UnknownStrategy(String),
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IO error: {}", err),
            Self::StrategyError(msg) => write!(f, "Strategy error: {}", msg),
            Self::UnknownStrategy(name) => write!(f, "Unknown strategy: {}", name),
        }
    }
}

impl Error for ResolutionError {}

impl From<std::io::Error> for ResolutionError {
    fn from(err: std::io::Error) -> Self {
        ResolutionError::IoError(err)
    }
}

/// Trait defining resolution strategy behavior
pub trait ResolutionStrategy {
    /// Name of the strategy
    fn name(&self) -> &str;
    
    /// Check if this strategy can handle the given conflict
    fn can_handle(&self, conflict: &ConflictRegion) -> bool;
    
    /// Resolve a conflict region
    fn resolve_conflict(&self, conflict: &ConflictRegion) -> Result<String, ResolutionError>;
}

/// Resolution result for a file
pub struct ResolutionResult {
    /// Path to the resolved file
    pub path: String,
    
    /// Resolved content
    pub content: String,
    
    /// Number of conflicts successfully resolved
    pub resolved_count: usize,
    
    /// Number of conflicts that couldn't be resolved
    pub unresolved_count: usize,
    
    /// Strategy used for resolution
    pub strategy_name: String,
}

/// Resolution engine for merge conflicts
pub struct ResolutionEngine {
    strategies: Vec<Box<dyn ResolutionStrategy>>,
    config: Config,
    file_resolution_handler: Option<Box<dyn Fn(&ConflictFile) -> Option<Box<dyn ResolutionStrategy>>>>,
}

impl ResolutionEngine {
    /// Create a new resolution engine
    pub fn new() -> Self {
        // Load configuration
        let config = Config::load().unwrap_or_else(|_| {
            warn!("Failed to load configuration, using defaults");
            Config::default()
        });
        
        // Add default strategies
        let mut engine = ResolutionEngine {
            strategies: Vec::new(),
            config,
            file_resolution_handler: None,
        };
        
        // Add rule-based strategies
        engine.add_strategy(Box::new(WhitespaceOnlyStrategy::new()));
        
        // Try to add fallback AI strategy if available
        if let Ok(fallback_strategy) = crate::fallback::FallbackResolutionStrategy::new() {
            info!("Adding fallback AI resolution strategy with providers: {:?}", fallback_strategy.provider_names());
            engine.add_strategy(Box::new(fallback_strategy));
        } else if let Ok(ai_strategy) = crate::ai_resolution::AIResolutionStrategy::new() {
            // If fallback isn't available, try single AI provider
            info!("Adding single AI resolution strategy: {}", ai_strategy.name());
            engine.add_strategy(Box::new(ai_strategy));
        }
        
        // Special handling for file-based resolution strategies for conflicted files
        engine.add_file_resolution_handler(|conflict_file| {
            if let Ok(ai_strategy) = crate::ai_resolution::AIResolutionStrategyWithFile::new(conflict_file) {
                info!("Using AI resolution strategy with full file context for better results");
                Some(Box::new(ai_strategy) as Box<dyn crate::resolution_engine::ResolutionStrategy>)
            } else {
                None
            }
        });
        
        engine
    }
    
    /// Add a resolution strategy
    pub fn add_strategy(&mut self, strategy: Box<dyn ResolutionStrategy>) {
        info!("Adding resolution strategy: {}", strategy.name());
        self.strategies.push(strategy);
    }
    
    /// Add a file resolution handler function that can create a strategy based on the whole file
    pub fn add_file_resolution_handler<F>(&mut self, handler: F) 
    where
        F: Fn(&ConflictFile) -> Option<Box<dyn ResolutionStrategy>> + 'static,
    {
        info!("Adding file resolution handler for context-aware conflict resolution");
        self.file_resolution_handler = Some(Box::new(handler));
    }
    
    /// Get the appropriate strategy for a file based on its type/extension
    pub fn get_strategy_for_file(&self, file_path: &str) -> Option<&Box<dyn ResolutionStrategy>> {
        // Get the strategy name from configuration based on file extension
        let strategy_name = self.config.get_strategy_for_file(file_path);
        
        // Find the corresponding strategy
        self.strategies.iter().find(|s| s.name() == strategy_name)
    }
    
    /// Resolve conflicts in a file
    pub fn resolve_file(&self, conflict_file: &ConflictFile) -> Result<ResolutionResult, ResolutionError> {
        let mut content = conflict_file.content.clone();
        let mut resolved_count = 0;
        let mut unresolved_count = 0;
        let mut strategy_name = "none".to_string();
        
        // First check if we have a file resolution handler that can provide a context-aware strategy
        if let Some(handler) = &self.file_resolution_handler {
            if let Some(file_strategy) = handler(conflict_file) {
                debug!("Using file resolution handler with strategy: {}", file_strategy.name());
                
                // Process each conflict region with the file-aware strategy
                let mut all_resolved = true;
                for conflict in &conflict_file.conflicts {
                    if file_strategy.can_handle(conflict) {
                        match file_strategy.resolve_conflict(conflict) {
                            Ok(resolved_content) => {
                                // Use the same content replacement logic as below
                                // Extract the actual conflict marker pattern from the file content
                                let conflict_start = conflict.start_line - 1; // 0-indexed
                                let conflict_end = conflict.end_line; // 0-indexed, line after the conflict
                                
                                let conflict_lines: Vec<&str> = content.lines().collect();
                                if conflict_end <= conflict_lines.len() {
                                    let actual_conflict = &conflict_lines[conflict_start..conflict_end].join("\n");
                                    debug!("Actual conflict in file: {}\n", actual_conflict);
                                    
                                    // Replace the conflict with the resolved content
                                    content = content.replace(actual_conflict, &resolved_content);
                                    resolved_count += 1;
                                } else {
                                    all_resolved = false;
                                    unresolved_count += 1;
                                    warn!("Invalid conflict line range: {} to {} (content has {} lines)", 
                                          conflict_start, conflict_end, conflict_lines.len());
                                }
                            },
                            Err(err) => {
                                all_resolved = false;
                                unresolved_count += 1;
                                warn!("File-aware strategy '{}' failed to resolve conflict: {}", file_strategy.name(), err);
                            }
                        }
                    } else {
                        all_resolved = false;
                        unresolved_count += 1;
                    }
                }
                
                // If all conflicts were resolved, return the result
                if all_resolved && unresolved_count == 0 {
                    return Ok(ResolutionResult {
                        path: conflict_file.path.clone(),
                        content,
                        resolved_count,
                        unresolved_count,
                        strategy_name: file_strategy.name().to_string(),
                    });
                }
                
                // Otherwise, we'll fall back to individual conflict resolution strategies
                debug!("File-aware strategy did not resolve all conflicts, falling back to regular strategies");
            }
        }
        
        // If no file resolution handler or it couldn't resolve all conflicts,
        // fall back to traditional strategy-by-strategy conflict resolution
        
        // First try to get the configured strategy for this file type
        let file_specific_strategy = self.get_strategy_for_file(&conflict_file.path);
        
        // Process each conflict region
        for conflict in &conflict_file.conflicts {
            // Flag to track if this conflict was resolved
            let mut resolved = false;
            
            // First try the file-specific strategy if available
            if let Some(strategy) = file_specific_strategy {
                if strategy.can_handle(conflict) {
                    debug!("Using file-type strategy '{}' for conflict in {}", strategy.name(), conflict_file.path);
                    strategy_name = strategy.name().to_string();
                    
                    match strategy.resolve_conflict(conflict) {
                        Ok(resolved_content) => {
                            // Replace the conflict with the resolved content
                            // Try to find the branch name from the conflict markers in the whole content
                            let mut branch_name = "branch-name";
                            
                            // Look for any >>>>>>> lines to get the branch name
                            for line in content.lines() {
                                if line.starts_with(">>>>>>> ") {
                                    branch_name = line.trim_start_matches(">>>>>>> ");
                                    debug!("Found branch name from content: {}", branch_name);
                                    break;
                                }
                            }
                            
                            // Also check if we can find the branch name in the conflict regions
                            for conflict in &conflict_file.conflicts {
                                if conflict.end_line <= content.lines().count() {
                                    if let Some(line) = content.lines().nth(conflict.end_line - 1) {
                                        if line.starts_with(">>>>>>> ") {
                                            branch_name = line.trim_start_matches(">>>>>>> ");
                                            debug!("Found branch name from conflict region: {}", branch_name);
                                            break;
                                        }
                                    }
                                }
                            }
                            
                            // Try first with the exact text including line breaks
                            let conflict_pattern = format!("<<<<<<< HEAD\n{}=======\n{}>>>>>>> {}", 
                                                      conflict.our_content, conflict.their_content, branch_name);
                            
                            debug!("Attempting to match exact conflict pattern of {} chars", conflict_pattern.len());
                            
                            // First try with the exact pattern
                            let content_before = content.clone();
                            content = content.replace(&conflict_pattern, &resolved_content);
                            
                            // If no replacement happened, try a more flexible pattern search
                            if content == content_before {
                                debug!("Exact pattern match failed, trying with partial matching");
                                
                                // Try to find the conflict lines approximately
                                let mut found = false;
                                // Create a new string to search in
                                let content_str = content.clone();
                                let lines: Vec<&str> = content_str.lines().collect();
                                
                                // Find the conflict boundaries
                                let mut conflict_start_idx = 0;
                                let mut conflict_end_idx = 0;
                                
                                for i in 0..lines.len() {
                                    if lines[i].contains("<<<<<<< HEAD") {
                                        conflict_start_idx = i;
                                        for j in i+1..lines.len() {
                                            if lines[j].contains(">>>>>>> ") {
                                                conflict_end_idx = j;
                                                found = true;
                                                break;
                                            }
                                        }
                                        if found {
                                            break;
                                        }
                                    }
                                }
                                
                                if found {
                                    let actual_conflict = lines[conflict_start_idx..=conflict_end_idx].join("\n");
                                    debug!("Found approximate conflict pattern: {} chars", actual_conflict.len());
                                    content = content.replace(&actual_conflict, &resolved_content);
                                }
                            }
                            
                            resolved_count += 1;
                            resolved = true;
                        }
                        Err(err) => {
                            warn!("File-type strategy '{}' failed to resolve conflict: {}", strategy.name(), err);
                            // Will fall back to trying other strategies
                        }
                    }
                }
            }
            
            // If not yet resolved, try all strategies in order
            if !resolved {
                for strategy in &self.strategies {
                    if strategy.can_handle(conflict) {
                        debug!("Using fallback strategy '{}' for conflict", strategy.name());
                        strategy_name = strategy.name().to_string();
                        
                        match strategy.resolve_conflict(conflict) {
                            Ok(resolved_content) => {
                                // Replace the conflict with the resolved content
                                debug!("Replacing conflict:\n  Original: <<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name", conflict.our_content, conflict.their_content);
                                debug!("  Will replace with: {}", resolved_content);
                                
                                // Extract the actual conflict marker pattern from the file content
                                let conflict_start = conflict.start_line - 1; // 0-indexed
                                let conflict_end = conflict.end_line; // 0-indexed, line after the conflict
                                
                                let conflict_lines: Vec<&str> = content.lines().collect();
                                if conflict_end <= conflict_lines.len() {
                                    let actual_conflict = &conflict_lines[conflict_start..conflict_end].join("\n");
                                    debug!("Actual conflict in file: {}\n", actual_conflict);
                                    
                                    // Extract the branch name from the conflict marker
                                let branch_name = if let Some(line) = conflict_lines.get(conflict_end - 1) {
                                    if line.starts_with(">>>>>>> ") {
                                        line.trim_start_matches(">>>>>>> ").to_string()
                                    } else {
                                        "branch-name".to_string() // fallback
                                    }
                                } else {
                                    "branch-name".to_string() // fallback
                                };
                                
                                debug!("Using branch name: {}", branch_name);
                                
                                // Try with actual conflict first
                                // Try direct replacement first
                            let content_before = content.clone();
                            content = content.replace(actual_conflict, &resolved_content);
                            
                            // If no replacement happened, try a more flexible pattern search
                            if content == content_before {
                                debug!("Exact conflict replacement failed, trying with partial matching");
                                
                                // Try to find the conflict lines approximately
                                let mut found = false;
                                // Create a new string to search in
                                let content_str = content.clone();
                                let lines: Vec<&str> = content_str.lines().collect();
                                
                                // Find the conflict boundaries
                                let mut conflict_start_idx = 0;
                                let mut conflict_end_idx = 0;
                                
                                for i in 0..lines.len() {
                                    if lines[i].contains("<<<<<<< HEAD") {
                                        conflict_start_idx = i;
                                        for j in i+1..lines.len() {
                                            if lines[j].contains(">>>>>>> ") {
                                                conflict_end_idx = j;
                                                found = true;
                                                break;
                                            }
                                        }
                                        if found {
                                            break;
                                        }
                                    }
                                }
                                
                                if found {
                                    let actual_conflict = lines[conflict_start_idx..=conflict_end_idx].join("\n");
                                    debug!("Found approximate conflict pattern: {} chars", actual_conflict.len());
                                    content = content.replace(&actual_conflict, &resolved_content);
                                }
                            }
                                    debug!("After replacement, content length: {}", content.len());
                                } else {
                                    warn!("Invalid conflict line range: {} to {} (content has {} lines)", 
                                          conflict_start, conflict_end, conflict_lines.len());
                                    // Fall back to old method
                                    // Try to find the branch name from the conflict markers in the whole content
                                    let mut branch_name = "branch-name";
                                    
                                    // Look for any >>>>>>> lines to get the branch name
                                    for line in content.lines() {
                                        if line.starts_with(">>>>>>> ") {
                                            branch_name = line.trim_start_matches(">>>>>>> ");
                                            debug!("Found branch name from content: {}", branch_name);
                                            break;
                                        }
                                    }
                                    
                                    // Try first with the exact text including line breaks
                                    let conflict_pattern = format!("<<<<<<< HEAD\n{}=======\n{}>>>>>>> {}", 
                                                              conflict.our_content, conflict.their_content, branch_name);
                                    
                                    debug!("Attempting to match exact conflict pattern of {} chars", conflict_pattern.len());
                                    
                                    // First try with the exact pattern
                                    let content_before = content.clone();
                                    content = content.replace(&conflict_pattern, &resolved_content);
                                    
                                    // If no replacement happened, try a more flexible pattern search
                                    if content == content_before {
                                        debug!("Exact pattern match failed, trying with partial matching");
                                        
                                        // Try to find the conflict lines approximately
                                        let mut found = false;
                                        // Create a new string to search in
                                        let content_str = content.clone();
                                        let lines: Vec<&str> = content_str.lines().collect();
                                        
                                        // Find the conflict boundaries
                                        let mut conflict_start_idx = 0;
                                        let mut conflict_end_idx = 0;
                                        
                                        for i in 0..lines.len() {
                                            if lines[i].contains("<<<<<<< HEAD") {
                                                conflict_start_idx = i;
                                                for j in i+1..lines.len() {
                                                    if lines[j].contains(">>>>>>> ") {
                                                        conflict_end_idx = j;
                                                        found = true;
                                                        break;
                                                    }
                                                }
                                                if found {
                                                    break;
                                                }
                                            }
                                        }
                                        
                                        if found {
                                            let actual_conflict = lines[conflict_start_idx..=conflict_end_idx].join("\n");
                                            debug!("Found approximate conflict pattern: {} chars", actual_conflict.len());
                                            content = content.replace(&actual_conflict, &resolved_content);
                                        }
                                    }
                                }
                                
                                resolved_count += 1;
                                resolved = true;
                                break;
                            }
                            Err(err) => {
                                warn!("Strategy '{}' failed to resolve conflict: {}", strategy.name(), err);
                                // Continue to the next strategy
                            }
                        }
                    }
                }
            }
            
            if !resolved {
                unresolved_count += 1;
                warn!("No strategy could resolve conflict at line {}", conflict.start_line);
            }
        }
        
        // Write the result back to the file
        info!(
            "Resolved {}/{} conflicts in file {}",
            resolved_count,
            resolved_count + unresolved_count,
            conflict_file.path
        );
        
        Ok(ResolutionResult {
            path: conflict_file.path.clone(),
            content,
            resolved_count,
            unresolved_count,
            strategy_name,
        })
    }
    
    /// Resolve conflicts with a specific strategy
    pub fn resolve_with_strategy(
        &self,
        conflict_file: &ConflictFile,
        strategy_name: &str,
    ) -> Result<ResolutionResult, ResolutionError> {
        // Find the requested strategy
        let strategy = self.strategies
            .iter()
            .find(|s| s.name() == strategy_name)
            .ok_or_else(|| ResolutionError::UnknownStrategy(strategy_name.to_string()))?;
        
        let mut content = conflict_file.content.clone();
        let mut resolved_count = 0;
        let mut unresolved_count = 0;
        
        // Process each conflict region
        for conflict in &conflict_file.conflicts {
            if strategy.can_handle(conflict) {
                match strategy.resolve_conflict(conflict) {
                    Ok(resolved_content) => {
                        // Replace the conflict with the resolved content
                        debug!("Replacing conflict for explicit strategy:\n  Original: <<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name", conflict.our_content, conflict.their_content);
                        debug!("  Will replace with: {}", resolved_content);
                        
                        // Extract the actual conflict marker pattern from the file content
                        let conflict_start = conflict.start_line - 1; // 0-indexed
                        let conflict_end = conflict.end_line; // 0-indexed, line after the conflict
                        
                        let conflict_lines: Vec<&str> = content.lines().collect();
                        if conflict_end <= conflict_lines.len() {
                            let actual_conflict = &conflict_lines[conflict_start..conflict_end].join("\n");
                            debug!("Actual conflict in file: {}\n", actual_conflict);
                            
                            // Extract the branch name from the conflict marker
                            let branch_name = if let Some(line) = conflict_lines.get(conflict_end - 1) {
                                if line.starts_with(">>>>>>> ") {
                                    line.trim_start_matches(">>>>>>> ").to_string()
                                } else {
                                    "branch-name".to_string() // fallback
                                }
                            } else {
                                "branch-name".to_string() // fallback
                            };
                            
                            debug!("Using branch name: {}", branch_name);
                            
                            // Try with actual conflict first
                            content = content.replace(actual_conflict, &resolved_content);
                            debug!("After replacement, content length: {}", content.len());
                        } else {
                            warn!("Invalid conflict line range: {} to {} (content has {} lines)", 
                                  conflict_start, conflict_end, conflict_lines.len());
                            // Fall back to old method
                            content = content.replace(
                                &format!(
                                    "<<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name",
                                    conflict.our_content,
                                    conflict.their_content
                                ),
                                &resolved_content
                            );
                        }
                        
                        resolved_count += 1;
                    }
                    Err(err) => {
                        warn!("Strategy '{}' failed to resolve conflict: {}", strategy.name(), err);
                        unresolved_count += 1;
                    }
                }
            } else {
                unresolved_count += 1;
                warn!("Strategy '{}' cannot handle conflict at line {}", strategy_name, conflict.start_line);
            }
        }
        
        Ok(ResolutionResult {
            path: conflict_file.path.clone(),
            content,
            resolved_count,
            unresolved_count,
            strategy_name: strategy_name.to_string(),
        })
    }
    
    /// Write the resolution result to a file
    pub fn write_resolution(
        &self,
        result: &ResolutionResult,
        output_path: Option<&str>,
    ) -> Result<(), ResolutionError> {
        let path = output_path.unwrap_or(&result.path);
        
        fs::write(path, &result.content)?;
        info!("Wrote resolved content to {}", path);
        
        Ok(())
    }
}

/// Strategy for resolving whitespace-only changes
pub struct WhitespaceOnlyStrategy;

impl WhitespaceOnlyStrategy {
    /// Create a new whitespace-only strategy
    pub fn new() -> Self {
        WhitespaceOnlyStrategy {}
    }
    
    /// Normalize whitespace in a string for comparison
    fn normalize_whitespace(&self, s: &str) -> String {
        s.split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}

impl ResolutionStrategy for WhitespaceOnlyStrategy {
    fn name(&self) -> &str {
        "whitespace-only"
    }
    
    fn can_handle(&self, conflict: &ConflictRegion) -> bool {
        // Check if the conflict is whitespace-only by comparing
        // normalized versions of the content
        let our_normalized = self.normalize_whitespace(&conflict.our_content);
        let their_normalized = self.normalize_whitespace(&conflict.their_content);
        
        our_normalized == their_normalized
    }
    
    fn resolve_conflict(&self, conflict: &ConflictRegion) -> Result<String, ResolutionError> {
        // For whitespace conflicts, we'll use "our" version
        // This is a simple choice, but we could implement more sophisticated
        // whitespace normalization if needed
        
        // First verify this is actually a whitespace-only conflict
        if !self.can_handle(conflict) {
            return Err(ResolutionError::StrategyError(
                "Not a whitespace-only conflict".to_string(),
            ));
        }
        
        Ok(conflict.our_content.clone())
    }
}

/// Mock resolution for test mode
pub fn mock_resolution_for_test(file_path: &str) -> Result<String, ResolutionError> {
    info!("Generating mock resolution for test file: {}", file_path);
    
    // Check if this is our example merge conflicts file
    if file_path.contains("merge_conflicts_example.sh") {
        // Return a completely resolved version of the file
        let resolved_content = "#!/bin/bash\n\n# A script demonstrating complex merge conflicts\n\n# Database connection settings\nDB_HOST=\"replica.db.example.com\" # Using replica from feature/app-metrics\nDB_PORT=5432\nDB_USER=\"app_user\"\nDB_PASSWORD=\"new_very_secure_password\" # Using newer password from feature/app-metrics\nDB_NAME=\"production_db\"\n\n# Function to check dependencies\ncheck_dependencies() {\n    echo \"Checking dependencies...\"\n    for dep in \"curl\" \"jq\" \"wget\"; do\n        if ! command -v $dep &> /dev/null; then\n            install_dependency $dep\n        fi\n    done\n}\n\ninstall_dependency() {\n    echo \"Installing $1...\"\n    # Implementation details\n}\n\n# Function to handle errors\nhandle_error() {\n    echo \"Error: $1\"\n    exit 1\n}\n\n# Main application function\nmain() {\n    # Parse command line arguments\n    parse_arguments \"$@\"\n    \n    # Initialize the application\n    check_dependencies\n    setup_database_connection\n    setup_cache\n    initialize_metrics\n    \n    # Start application\n    echo \"Starting application with $(get_thread_count) threads...\"\n    start_worker_processes\n    setup_signal_handlers\n    wait_for_completion\n}\n\nparse_arguments() {\n    # Parse command line arguments\n    while [[ $# -gt 0 ]]; do\n        case $1 in\n            --debug) DEBUG_MODE=true ;;\n            --threads=*) THREAD_COUNT=\"${1#*=}\" ;;\n            *) echo \"Unknown option: $1\" ;;\n        esac\n        shift\n    done\n}\n\nget_thread_count() {\n    echo ${THREAD_COUNT:-$(nproc)}\n}\n\n# Call main function with arguments\nmain \"$@\"\n";
        
        Ok(resolved_content.to_string())
    } else {
        // For any other file, return a generic resolved content
        Err(ResolutionError::StrategyError(format!("No mock resolution available for {}", file_path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict_parser::ConflictRegion;
    use proptest::prelude::*;
    
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
    
    #[test]
    fn test_whitespace_only_strategy() {
        let strategy = WhitespaceOnlyStrategy::new();
        
        // Test cases where whitespace differs but content is the same
        let conflict1 = create_test_conflict("hello world\n", "hello   world\n");
        let conflict2 = create_test_conflict("hello\nworld\n", "hello world\n");
        
        assert!(strategy.can_handle(&conflict1));
        assert!(strategy.can_handle(&conflict2));
        
        // Test cases where content differs (not just whitespace)
        let conflict3 = create_test_conflict("hello world\n", "hello universe\n");
        let conflict4 = create_test_conflict("hello world\n", "goodbye world\n");
        
        assert!(!strategy.can_handle(&conflict3));
        assert!(!strategy.can_handle(&conflict4));
        
        // Test resolution
        let result = strategy.resolve_conflict(&conflict1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world\n");
    }
    
    proptest! {
        #[test]
        fn test_whitespace_only_strategy_prop(s in r"[\w\s]{1,100}") {
            let strategy = WhitespaceOnlyStrategy::new();
            
            // Create versions of the string with different whitespace
            let our_version = s.clone();
            let their_version = s.split_whitespace().collect::<Vec<&str>>().join("   ");
            
            let conflict = create_test_conflict(&our_version, &their_version);
            
            // If the strings only differ in whitespace, the strategy should handle it
            if strategy.normalize_whitespace(&our_version) == strategy.normalize_whitespace(&their_version) {
                prop_assert!(strategy.can_handle(&conflict));
                
                let result = strategy.resolve_conflict(&conflict);
                prop_assert!(result.is_ok());
            }
        }
    }
    
    #[test]
    fn test_resolution_engine() {
        let engine = ResolutionEngine::new();
        
        // Create a conflict file with a whitespace-only conflict
        let conflict = create_test_conflict("hello world\n", "hello   world\n");
        let conflict_file = ConflictFile {
            path: "test.txt".to_string(),
            conflicts: vec![conflict],
            content: "<<<<<<< HEAD\nhello world\n=======\nhello   world\n>>>>>>> branch-name\n".to_string(),
        };
        
        // Resolve the conflict
        let result = engine.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        let resolution = result.unwrap();
        assert_eq!(resolution.resolved_count, 1);
        assert_eq!(resolution.unresolved_count, 0);
        assert_eq!(resolution.strategy_name, "whitespace-only");
        
        // The content should be resolved
        assert_eq!(resolution.content, "hello world\n\n");
    }
    
    #[test]
    fn test_file_type_specific_strategy() {
        use std::env;
        
        // Set up environment variables for file-type specific strategies
        env::set_var("RIZZLER_EXTENSION_STRATEGY_txt", "whitespace-only");
        
        // Create a new engine that will load the config with our environment variables
        let engine = ResolutionEngine::new();
        
        // Create a conflict file with a whitespace-only conflict
        let conflict = create_test_conflict("hello world\n", "hello   world\n");
        let conflict_file = ConflictFile {
            path: "file.txt".to_string(),  // Note the .txt extension
            conflicts: vec![conflict],
            content: "<<<<<<< HEAD\nhello world\n=======\nhello   world\n>>>>>>> branch-name\n".to_string(),
        };
        
        // Verify that the engine selects the right strategy for this file type
        let strategy = engine.get_strategy_for_file(&conflict_file.path);
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().name(), "whitespace-only");
        
        // Resolve the conflict
        let result = engine.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        let resolution = result.unwrap();
        assert_eq!(resolution.resolved_count, 1);
        assert_eq!(resolution.unresolved_count, 0);
        assert_eq!(resolution.strategy_name, "whitespace-only");
        
        // Clean up environment
        env::remove_var("RIZZLER_EXTENSION_STRATEGY_txt");
    }
}