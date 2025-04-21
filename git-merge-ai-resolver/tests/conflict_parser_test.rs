// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use git_merge_ai_resolver::conflict_parser::{parse_conflict_file, parse_conflict_file_with_base, ConflictParseError};
use std::fs::File;
use std::io::Write;

#[test]
fn test_conflict_parser_with_base() {
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
    
    // Parse the conflict file with base content
    let result = parse_conflict_file_with_base(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    assert!(result.is_ok());
    
    let conflict_file = result.unwrap();
    assert_eq!(conflict_file.conflicts.len(), 1);
    
    let conflict = &conflict_file.conflicts[0];
    assert_eq!(conflict.base_content, base_content);
    assert_eq!(conflict.our_content, "This is our content.\n");
    assert_eq!(conflict.their_content, "This is their content.\n");
}

#[test]
fn test_parse_conflict_file_no_base() {
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
    assert_eq!(conflict.base_content, ""); // Base content should be empty
    assert_eq!(conflict.our_content, "This is our content.\n");
    assert_eq!(conflict.their_content, "This is their content.\n");
    assert_eq!(conflict.start_line, 2);
    assert_eq!(conflict.end_line, 6);
}