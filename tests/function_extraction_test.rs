// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use rizzler::conflict_parser::{parse_conflict_file_with_context_matching};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_extract_function_directly() {
    let conflict_content = r#"<<<<<<< HEAD
function calculateValue(raw) {
    const factor = 2.0; // We increased the factor
    return raw * factor;
}
=======
function calculateValue(raw) {
    const factor = 1.5;
    const offset = 10; // Added an offset
    return (raw * factor) + offset;
}
>>>>>>> branch-name"#;
    
    let base_content = r#"function calculateValue(raw) {
    const factor = 1.5;
    return raw * factor;
}"#;
    
    // Create a temporary file to store the content
    let temp_dir = tempdir().unwrap();
    let conflict_path = temp_dir.path().join("conflict_func.txt");
    let base_path = temp_dir.path().join("base_func.txt");
    
    File::create(&conflict_path)
        .unwrap()
        .write_all(conflict_content.as_bytes())
        .unwrap();
    
    File::create(&base_path)
        .unwrap()
        .write_all(base_content.as_bytes())
        .unwrap();
    
    println!("Running simplified test with calculateValue function");
    println!("Base content:\n{}", base_content);
    println!("Conflict content:\n{}", conflict_content);
    
    // Parse the conflict directly
    let result = parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(), 
        base_path.to_str().unwrap()
    );
    
    // Verify results
    assert!(result.is_ok());
    let conflict_file = result.unwrap();
    
    // Validate the contents
    assert_eq!(conflict_file.conflicts.len(), 1);
    println!("Base content from parser: {}", conflict_file.conflicts[0].base_content);
    assert!(conflict_file.conflicts[0].base_content.contains("function calculateValue"));
}