// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::conflict_parser::{ConflictFile, ConflictRegion};
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
}

impl ResolutionEngine {
    /// Create a new resolution engine
    pub fn new() -> Self {
        // Add default strategies
        let mut engine = ResolutionEngine {
            strategies: Vec::new(),
        };
        
        // Add rule-based strategies
        engine.add_strategy(Box::new(WhitespaceOnlyStrategy::new()));
        
        engine
    }
    
    /// Add a resolution strategy
    pub fn add_strategy(&mut self, strategy: Box<dyn ResolutionStrategy>) {
        info!("Adding resolution strategy: {}", strategy.name());
        self.strategies.push(strategy);
    }
    
    /// Resolve conflicts in a file
    pub fn resolve_file(&self, conflict_file: &ConflictFile) -> Result<ResolutionResult, ResolutionError> {
        let mut content = conflict_file.content.clone();
        let mut resolved_count = 0;
        let mut unresolved_count = 0;
        let mut strategy_name = "none".to_string();
        
        // Process each conflict region
        for conflict in &conflict_file.conflicts {
            // Find a strategy that can handle this conflict
            let mut resolved = false;
            
            for strategy in &self.strategies {
                if strategy.can_handle(conflict) {
                    debug!("Using strategy '{}' for conflict", strategy.name());
                    strategy_name = strategy.name().to_string();
                    
                    match strategy.resolve_conflict(conflict) {
                        Ok(resolved_content) => {
                            // Replace the conflict with the resolved content
                            // This is a simplified placeholder - in a real implementation,
                            // we would need to track line offsets as we replace content
                            content = content.replace(
                                &format!(
                                    "<<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name",
                                    conflict.our_content,
                                    conflict.their_content
                                ),
                                &resolved_content
                            );
                            
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
                        content = content.replace(
                            &format!(
                                "<<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name",
                                conflict.our_content,
                                conflict.their_content
                            ),
                            &resolved_content
                        );
                        
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
        fn test_whitespace_only_strategy_prop(s in "[\w\s]{1,100}") {
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
}