// A simplified version of the conflict parser for testing

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

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

/// Parse conflicts from a file with Git conflict markers and include base content from the ancestor file
/// 
/// This function enhances conflict parsing by loading base content from the ancestor file that Git provides
/// as part of the merge driver interface.
/// 
/// * `conflict_path` - Path to the file with conflict markers
/// * `base_path` - Path to the base/ancestor version of the file
pub fn parse_conflict_file_with_base(conflict_path: &str, base_path: &str) -> Result<ConflictFile, ConflictParseError> {
    println!("Parsing conflict file with base content. Conflict: {}, Base: {}", conflict_path, base_path);
    
    // First, parse the conflict file normally
    let mut conflict_file = parse_conflict_file(conflict_path)?;
    
    // Read the base file
    let base_file = File::open(base_path).map_err(|e| {
        eprintln!("Failed to open base file: {}", e);
        ConflictParseError::IoError(e)
    })?;
    
    let base_content = std::io::read_to_string(base_file).map_err(|e| {
        eprintln!("Failed to read base file content: {}", e);
        ConflictParseError::IoError(e)
    })?;
    
    // Update each conflict region with the base content
    // For simplicity, we're using the entire base file content for each conflict
    // In a more sophisticated implementation, we might want to match specific sections
    for conflict in &mut conflict_file.conflicts {
        conflict.base_content = base_content.clone();
    }
    
    println!("Successfully parsed conflict file with base content. Found {} conflicts", conflict_file.conflicts.len());
    Ok(conflict_file)
}

fn main() {
    // Create temporary files for testing
    use std::io::Write;
    
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
    
    // Parse the conflict file with base content
    let result = parse_conflict_file_with_base(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    match result {
        Ok(file) => {
            println!("Successfully parsed conflict file!");
            println!("Found {} conflicts", file.conflicts.len());
            
            for (i, conflict) in file.conflicts.iter().enumerate() {
                println!("Conflict #{}:", i+1);
                println!("Base content: {}", conflict.base_content);
                println!("Our content: {}", conflict.our_content);
                println!("Their content: {}", conflict.their_content);
                println!("Start line: {}", conflict.start_line);
                println!("End line: {}", conflict.end_line);
            }
        },
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    }
}