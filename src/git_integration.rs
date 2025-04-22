// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use std::path::Path;
use tracing::{debug, info, error, warn};

/// Represents the paths provided by Git to the merge driver
pub struct MergeDriverPaths {
    /// Path to the ancestor's version of the file
    pub ancestor_path: String,
    
    /// Path to the current version of the file
    pub current_path: String,
    
    /// Path to the other branches' version of the file
    pub other_path: String,
    
    /// Path to the file with conflict markers
    pub conflict_path: String,
}

/// Error types for Git merge driver operations
#[derive(Debug)]
pub enum MergeDriverError {
    /// Invalid number of arguments
    InvalidArgumentCount,
    
    /// File not found
    FileNotFound(String),
    
    /// IO error during file operations
    IoError(std::io::Error),
}

impl std::fmt::Display for MergeDriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgumentCount => write!(f, "Invalid number of arguments for Git merge driver"),
            Self::FileNotFound(path) => write!(f, "File not found: {}", path),
            Self::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for MergeDriverError {}

/// Parse Git merge driver arguments
///
/// Git calls merge drivers with: git-merge-ai-resolver %O %A %B %P
/// Where:
/// - %O: Path to the ancestor's version of the file
/// - %A: Path to the current version of the file
/// - %B: Path to the other branches' version of the file
/// - %P: Path to the file with conflict markers
pub fn parse_merge_driver_args(args: &[String]) -> Result<MergeDriverPaths, MergeDriverError> {
    if args.len() < 4 {
        return Err(MergeDriverError::InvalidArgumentCount);
    }
    
    // Extract the paths from the arguments
    let ancestor_path = &args[0];
    let current_path = &args[1];
    let other_path = &args[2];
    let conflict_path = &args[3];
    
    // Validate that the paths exist
    for path in [ancestor_path, current_path, other_path, conflict_path] {
        if !Path::new(path).exists() {
            return Err(MergeDriverError::FileNotFound(path.clone()));
        }
    }
    
    debug!("Found valid merge driver paths");
    info!("Ancestor: {}, Current: {}, Other: {}, Conflict: {}", 
          ancestor_path, current_path, other_path, conflict_path);
    
    Ok(MergeDriverPaths {
        ancestor_path: ancestor_path.clone(),
        current_path: current_path.clone(),
        other_path: other_path.clone(),
        conflict_path: conflict_path.clone(),
    })
}

/// Process the merge as a Git merge driver
///
/// Returns an exit code that will be passed back to Git:
/// - 0: Success - conflicts resolved
/// - Non-zero: Failure - manual resolution needed
pub fn process_merge(paths: &MergeDriverPaths) -> i32 {
    use crate::conflict_parser;
    use crate::resolution_engine::ResolutionEngine;
    
    info!("Processing merge for file: {}", paths.conflict_path);
    
    // Parse the conflict file with base content from ancestor
    let conflict_file = match conflict_parser::parse_conflict_file_with_base(
        &paths.conflict_path, 
        &paths.ancestor_path
    ) {
        Ok(file) => file,
        Err(err) => {
            error!("Failed to parse conflict file: {}", err);
            // Fallback to basic parser if enhanced parser fails
            match conflict_parser::parse_conflict_file(&paths.conflict_path) {
                Ok(file) => {
                    warn!("Falling back to basic conflict parser without base content");
                    file
                },
                Err(err) => {
                    error!("Also failed with basic parser: {}", err);
                    return 1; // Return failure
                }
            }
        }
    };
    
    // Create a resolution engine
    let engine = ResolutionEngine::new();
    
    // Resolve conflicts
    let resolution_result = match engine.resolve_file(&conflict_file) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to resolve conflicts: {}", err);
            return 1; // Return failure
        }
    };
    
    // Check if all conflicts were resolved
    if resolution_result.unresolved_count > 0 {
        warn!(
            "Not all conflicts were resolved: {} unresolved, {} resolved",
            resolution_result.unresolved_count,
            resolution_result.resolved_count
        );
        return 1; // Return failure if any conflicts weren't resolved
    }
    
    // Write the resolved content back to the file
    match engine.write_resolution(&resolution_result, Some(&paths.conflict_path)) {
        Ok(_) => {
            info!("Successfully resolved all conflicts in {}", paths.conflict_path);
            0 // Return success
        }
        Err(err) => {
            error!("Failed to write resolved content: {}", err);
            1 // Return failure
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs::File;
    use std::io::Write;
    
    #[test]
    fn test_parse_merge_driver_args_with_missing_args() {
        let args = vec!["path1".to_string()];
        let result = parse_merge_driver_args(&args);
        assert!(matches!(result, Err(MergeDriverError::InvalidArgumentCount)));
    }
    
    proptest! {
        #[test]
        fn test_parse_merge_driver_args_prop(ancestor: String, current: String, other: String, conflict: String) {
            // Create temp files for testing
            let temp_dir = tempfile::tempdir().unwrap();
            
            let ancestor_path = temp_dir.path().join(&ancestor);
            let current_path = temp_dir.path().join(&current);
            let other_path = temp_dir.path().join(&other);
            let conflict_path = temp_dir.path().join(&conflict);
            
            // Create the files
            File::create(&ancestor_path).unwrap().write_all(b"ancestor").unwrap();
            File::create(&current_path).unwrap().write_all(b"current").unwrap();
            File::create(&other_path).unwrap().write_all(b"other").unwrap();
            File::create(&conflict_path).unwrap().write_all(b"conflict").unwrap();
            
            let args = vec![
                ancestor_path.to_str().unwrap().to_string(),
                current_path.to_str().unwrap().to_string(),
                other_path.to_str().unwrap().to_string(),
                conflict_path.to_str().unwrap().to_string(),
            ];
            
            let result = parse_merge_driver_args(&args);
            prop_assert!(result.is_ok());
            
            let paths = result.unwrap();
            prop_assert_eq!(&paths.ancestor_path, &args[0]);
            prop_assert_eq!(&paths.current_path, &args[1]);
            prop_assert_eq!(&paths.other_path, &args[2]);
            prop_assert_eq!(&paths.conflict_path, &args[3]);
        }
    }
}