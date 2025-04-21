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
                base_content: String::new(), // We don't have base content from markers
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
    
    proptest! {
        #[test]
        fn test_parse_conflict_file_prop(our_content in "[\w\s]{1,100}", their_content in "[\w\s]{1,100}") {
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
            prop_assert_eq!(conflict.our_content, format!("{}
", our_content));
            prop_assert_eq!(conflict.their_content, format!("{}
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