// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use tracing::{debug, info, warn};

/// A representation of a conflict region in a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRegion {
    /// Content from the base version
    pub base_content: String,
    
    /// Content from "our" version (current branch)
    pub our_content: String,
    
    /// Content from "their" version (other branch)
    pub their_content: String,
    
    /// Start line number in the conflict file
    pub start_line: usize,
    
    /// End line number in the conflict file
    pub end_line: usize,
}

/// A file with one or more conflict regions
#[derive(Debug)]
pub struct ConflictFile {
    /// Path to the file
    pub path: String,
    
    /// Conflict regions in the file
    pub conflicts: Vec<ConflictRegion>,
    
    /// Complete file content with conflict markers
    pub content: String,
}

/// Error types for conflict parsing operations
#[derive(Debug)]
pub enum ConflictParseError {
    /// IO error during file operations
    IoError(io::Error),
    
    /// Invalid conflict markers
    InvalidConflictMarkers(String),
    
    /// No conflicts found
    NoConflictsFound,
}

impl std::fmt::Display for ConflictParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IO error: {}", err),
            Self::InvalidConflictMarkers(details) => write!(f, "Invalid conflict markers: {}", details),
            Self::NoConflictsFound => write!(f, "No conflicts found in file"),
        }
    }
}

impl std::error::Error for ConflictParseError {}

impl From<io::Error> for ConflictParseError {
    fn from(err: io::Error) -> Self {
        ConflictParseError::IoError(err)
    }
}

/// Parse conflicts from a file with Git conflict markers
pub fn parse_conflict_file(path: &str) -> Result<ConflictFile, ConflictParseError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    let mut content = String::new();
    let mut conflicts = Vec::new();
    
    let mut in_conflict = false;
    let mut conflict_start = 0;
    let mut our_content = String::new();
    let mut their_content = String::new();
    let mut line_number = 0;
    
    for line_result in reader.lines() {
        let line = line_result?;
        content.push_str(&line);
        content.push('\n');
        line_number += 1;
        
        if line.starts_with("<<<<<<<") {
            if in_conflict {
                return Err(ConflictParseError::InvalidConflictMarkers(
                    "Found nested conflict marker".to_string(),
                ));
            }
            in_conflict = true;
            conflict_start = line_number;
            continue;
        }
        
        if line.starts_with("=======") && in_conflict {
            // End of "our" content, start of "their" content
            continue;
        }
        
        if line.starts_with(">>>>>>>") && in_conflict {
            in_conflict = false;
            
            // Create a conflict region
            conflicts.push(ConflictRegion {
                base_content: String::new(), // Base content will be populated by parse_conflict_file_with_base
                our_content,
                their_content,
                start_line: conflict_start,
                end_line: line_number,
            });
            
            our_content = String::new();
            their_content = String::new();
            continue;
        }
        
        if in_conflict {
            // We're in a conflict region, add the line to the appropriate content
            if our_content.is_empty() && their_content.is_empty() {
                // We haven't seen the separator yet, so this is "our" content
                our_content.push_str(&line);
                our_content.push('\n');
            } else {
                // We've seen the separator, so this is "their" content
                their_content.push_str(&line);
                their_content.push('\n');
            }
        }
    }
    
    if in_conflict {
        return Err(ConflictParseError::InvalidConflictMarkers(
            "Unmatched conflict marker".to_string(),
        ));
    }
    
    if conflicts.is_empty() {
        return Err(ConflictParseError::NoConflictsFound);
    }
    
    Ok(ConflictFile {
        path: path.to_string(),
        conflicts,
        content,
    })
}

/// Parse conflicts from a file with Git conflict markers and include base content from the ancestor file
/// 
/// This function enhances conflict parsing by loading base content from the ancestor file that Git provides
/// as part of the merge driver interface.
/// 
/// * `conflict_path` - Path to the file with conflict markers
/// * `base_path` - Path to the base/ancestor version of the file
pub fn parse_conflict_file_with_base(conflict_path: &str, base_path: &str) -> Result<ConflictFile, ConflictParseError> {
    debug!("Parsing conflict file with base content. Conflict: {}, Base: {}", conflict_path, base_path);
    
    // First, parse the conflict file normally
    let mut conflict_file = parse_conflict_file(conflict_path)?;
    
    // Read the base file
    let base_file = File::open(base_path).map_err(|e| {
        warn!("Failed to open base file: {}", e);
        ConflictParseError::IoError(e)
    })?;
    
    let base_content = std::io::read_to_string(base_file).map_err(|e| {
        warn!("Failed to read base file content: {}", e);
        ConflictParseError::IoError(e)
    })?;
    
    // Update each conflict region with the base content
    // For simplicity, we're using the entire base file content for each conflict
    // In a more sophisticated implementation, we might want to match specific sections
    for conflict in &mut conflict_file.conflicts {
        conflict.base_content = base_content.clone();
    }
    
    info!("Successfully parsed conflict file with base content. Found {} conflicts", conflict_file.conflicts.len());
    Ok(conflict_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use proptest::prelude::*;
    
    #[test]
    fn test_parse_conflict_file_simple() {
        // Create a temporary file with a simple conflict
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("conflict.txt");
        
        let conflict_content = r#"This is a file with a conflict.
<<<<<<< HEAD
This is our content.
=======
This is their content.
>>>>>>> branch-name
This is after the conflict.
"#;
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(conflict_content.as_bytes()).unwrap();
        
        // Parse the conflict file
        let result = parse_conflict_file(file_path.to_str().unwrap());
        assert!(result.is_ok());
        
        let conflict_file = result.unwrap();
        assert_eq!(conflict_file.conflicts.len(), 1);
        
        let conflict = &conflict_file.conflicts[0];
        assert_eq!(conflict.our_content, "This is our content.\n");
        assert_eq!(conflict.their_content, "This is their content.\n");
        assert_eq!(conflict.start_line, 2);
        assert_eq!(conflict.end_line, 6);
    }
    
    #[test]
    fn test_parse_conflict_file_with_base() {
        // Create temporary files for testing
        let temp_dir = tempfile::tempdir().unwrap();
        
        // Create base file
        let base_path = temp_dir.path().join("base.txt");
        let base_content = "This is the base content.\n";
        let mut base_file = File::create(&base_path).unwrap();
        base_file.write_all(base_content.as_bytes()).unwrap();
        
        // Create conflict file
        let conflict_path = temp_dir.path().join("conflict.txt");
        let conflict_content = r#"This is a file with a conflict.
<<<<<<< HEAD
This is our content.
=======
This is their content.
>>>>>>> branch-name
This is after the conflict.
"#;
        let mut conflict_file = File::create(&conflict_path).unwrap();
        conflict_file.write_all(conflict_content.as_bytes()).unwrap();
        
        // Create files for current and other branches
        let current_path = temp_dir.path().join("current.txt");
        File::create(&current_path).unwrap().write_all(b"Current branch content").unwrap();
        
        let other_path = temp_dir.path().join("other.txt");
        File::create(&other_path).unwrap().write_all(b"Other branch content").unwrap();
        
        // Create merge driver paths
        let paths = crate::git_integration::MergeDriverPaths {
            ancestor_path: base_path.to_str().unwrap().to_string(),
            current_path: current_path.to_str().unwrap().to_string(),
            other_path: other_path.to_str().unwrap().to_string(),
            conflict_path: conflict_path.to_str().unwrap().to_string(),
        };
        
        // Parse the conflict file with base content
        let result = parse_conflict_file_with_base(paths.conflict_path.as_str(), paths.ancestor_path.as_str());
        assert!(result.is_ok());
        
        let conflict_file = result.unwrap();
        assert_eq!(conflict_file.conflicts.len(), 1);
        
        let conflict = &conflict_file.conflicts[0];
        assert_eq!(conflict.base_content, "This is the base content.\n");
        assert_eq!(conflict.our_content, "This is our content.\n");
        assert_eq!(conflict.their_content, "This is their content.\n");
    }
    
    proptest! {
        #[test]
        fn test_parse_conflict_file_prop(our_content in r"[\w\s]{1,100}", their_content in r"[\w\s]{1,100}") {
            let temp_dir = tempfile::tempdir().unwrap();
            let file_path = temp_dir.path().join("conflict.txt");
            
            let conflict_content = format!("\
Before the conflict.
<<<<<<< HEAD
{}
=======
{}
>>>>>>> branch-name
After the conflict.", our_content, their_content);
            
            let mut file = File::create(&file_path).unwrap();
            file.write_all(conflict_content.as_bytes()).unwrap();
            
            // Parse the conflict file
            let result = parse_conflict_file(file_path.to_str().unwrap());
            prop_assert!(result.is_ok());
            
            let conflict_file = result.unwrap();
            prop_assert_eq!(conflict_file.conflicts.len(), 1);
            
            let conflict = &conflict_file.conflicts[0];
            prop_assert_eq!(&conflict.our_content, &format!("{}
", our_content));
            prop_assert_eq!(&conflict.their_content, &format!("{}
", their_content));
        }
        
        #[test]
        fn test_parse_conflict_file_with_base_prop(base_content in r"[\w\s]{1,100}", our_content in r"[\w\s]{1,100}", their_content in r"[\w\s]{1,100}") {
            let temp_dir = tempfile::tempdir().unwrap();
            
            // Create base file
            let base_path = temp_dir.path().join("base.txt");
            let base_content_str = format!("{}
", base_content);
            let mut base_file = File::create(&base_path).unwrap();
            base_file.write_all(base_content_str.as_bytes()).unwrap();
            
            // Create conflict file
            let conflict_path = temp_dir.path().join("conflict.txt");
            let conflict_content = format!("Before the conflict.
<<<<<<< HEAD
{}
=======
{}
>>>>>>> branch-name
After the conflict.", our_content, their_content);
            let mut conflict_file = File::create(&conflict_path).unwrap();
            conflict_file.write_all(conflict_content.as_bytes()).unwrap();
            
            // Parse the conflict file with base content
            let result = parse_conflict_file_with_base(
                conflict_path.to_str().unwrap(),
                base_path.to_str().unwrap()
            );
            prop_assert!(result.is_ok());
            
            let conflict_file = result.unwrap();
            prop_assert_eq!(conflict_file.conflicts.len(), 1);
            
            let conflict = &conflict_file.conflicts[0];
            prop_assert_eq!(&conflict.base_content, &base_content_str);
            prop_assert_eq!(&conflict.our_content, &format!("{}
", our_content));
            prop_assert_eq!(&conflict.their_content, &format!("{}
", their_content));
        }
    }
    
    #[test]
    fn test_parse_conflict_file_multiple_conflicts() {
        // Create a temporary file with multiple conflicts
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("conflict.txt");
        
        let conflict_content = r#"This is a file with multiple conflicts.
<<<<<<< HEAD
Our content 1.
=======
Their content 1.
>>>>>>> branch-name
Between conflicts.
<<<<<<< HEAD
Our content 2.
=======
Their content 2.
>>>>>>> branch-name
After all conflicts.
"#;
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(conflict_content.as_bytes()).unwrap();
        
        // Parse the conflict file
        let result = parse_conflict_file(file_path.to_str().unwrap());
        assert!(result.is_ok());
        
        let conflict_file = result.unwrap();
        assert_eq!(conflict_file.conflicts.len(), 2);
        
        let conflict1 = &conflict_file.conflicts[0];
        assert_eq!(conflict1.our_content, "Our content 1.\n");
        assert_eq!(conflict1.their_content, "Their content 1.\n");
        
        let conflict2 = &conflict_file.conflicts[1];
        assert_eq!(conflict2.our_content, "Our content 2.\n");
        assert_eq!(conflict2.their_content, "Their content 2.\n");
    }
    
    #[test]
    fn test_parse_conflict_file_invalid_markers() {
        // Create a temporary file with invalid conflict markers
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("conflict.txt");
        
        let conflict_content = r#"This is a file with invalid conflict markers.
<<<<<<< HEAD
Our content.
"#; // Missing closing marker
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(conflict_content.as_bytes()).unwrap();
        
        // Parse the conflict file
        let result = parse_conflict_file(file_path.to_str().unwrap());
        assert!(result.is_err());
        
        match result {
            Err(ConflictParseError::InvalidConflictMarkers(_)) => (),
            _ => panic!("Expected InvalidConflictMarkers error"),
        }
    }
    
    #[test]
    fn test_parse_conflict_file_no_conflicts() {
        // Create a temporary file with no conflicts
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("no_conflict.txt");
        
        let content = "This is a file with no conflicts.\n";
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        
        // Parse the file
        let result = parse_conflict_file(file_path.to_str().unwrap());
        assert!(result.is_err());
        
        match result {
            Err(ConflictParseError::NoConflictsFound) => (),
            _ => panic!("Expected NoConflictsFound error"),
        }
    }
}