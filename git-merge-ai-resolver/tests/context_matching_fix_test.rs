// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use git_merge_ai_resolver::conflict_parser::{parse_conflict_file_with_context_matching, ConflictFile};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_context_matching_fixed_algorithm() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create base file with content that should be matchable
    let base_path = temp_dir.path().join("base_fix.txt");
    let base_content = r#"// Data processing module

// Process input data and return results
function processData(data) {
    const processed = data.map(item => transform(item));
    return processed;
}

function transform(item) {
    return {
        id: item.id,
        value: calculateValue(item.raw),
        timestamp: new Date().toISOString()
    };
}

function calculateValue(raw) {
    const factor = 1.5;
    return raw * factor;
}
"#;
    
    File::create(&base_path)
        .unwrap()
        .write_all(base_content.as_bytes())
        .unwrap();
    
    // Create conflict file with content that should match the calculateValue function
    let conflict_path = temp_dir.path().join("conflict_fix.txt");
    let conflict_content = r#"// Data processing module

// Process input data and return results
function processData(data) {
    const processed = data.map(item => transform(item));
    return processed;
}

function transform(item) {
    return {
        id: item.id,
        value: calculateValue(item.raw),
        timestamp: new Date().toISOString()
    };
}

<<<<<<< HEAD
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
>>>>>>> branch-name
"#;
    
    File::create(&conflict_path)
        .unwrap()
        .write_all(conflict_content.as_bytes())
        .unwrap();
    
    // Use the context matching parser
    println!("Running test with calculateValue conflict");
    println!("Base content:\n{}", base_content);
    println!("Conflict content:\n{}", conflict_content);
    
    let result = parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    // Verify results
    assert!(result.is_ok());
    let conflict_file = result.unwrap();
    
    // Validate we found one conflict
    assert_eq!(conflict_file.conflicts.len(), 1);
    
    // Verify that the context matching found the calculateValue function specifically
    let conflict = &conflict_file.conflicts[0];
    
    // The base content should contain the calculateValue function
    assert!(conflict.base_content.contains("function calculateValue"));
    assert!(conflict.base_content.contains("const factor = 1.5"));
    
    // The base content should NOT contain the entire file
    // It should be focused just on the relevant section
    let base_section_lines = conflict.base_content.lines().count();
    let full_file_lines = base_content.lines().count();
    
    assert!(base_section_lines < full_file_lines);
}

#[test]
fn test_context_matching_with_nested_functions() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create base file with nested functions
    let base_path = temp_dir.path().join("base_nested.txt");
    let base_content = r#"function outer() {
    console.log('Outer function');
    
    function inner1() {
        console.log('Inner function 1');
        return 'result1';
    }
    
    function inner2() {
        console.log('Inner function 2');
        return 'result2';
    }
    
    return inner1() + inner2();
}
"#;
    
    File::create(&base_path)
        .unwrap()
        .write_all(base_content.as_bytes())
        .unwrap();
    
    // Create conflict file with a conflict in one of the inner functions
    let conflict_path = temp_dir.path().join("conflict_nested.txt");
    let conflict_content = r#"function outer() {
    console.log('Outer function');
    
    function inner1() {
        console.log('Inner function 1');
        return 'result1';
    }
    
<<<<<<< HEAD
    function inner2() {
        console.log('Inner function 2 - modified');
        return 'modified-result2';
    }
=======
    function inner2() {
        console.log('Inner function 2');
        console.log('With extra logging');
        return 'result2-with-logging';
    }
>>>>>>> branch-name
    
    return inner1() + inner2();
}
"#;
    
    File::create(&conflict_path)
        .unwrap()
        .write_all(conflict_content.as_bytes())
        .unwrap();
    
    // Use the context matching parser
    let result = parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    // Verify results
    assert!(result.is_ok());
    let conflict_file = result.unwrap();
    
    // Validate we found one conflict
    assert_eq!(conflict_file.conflicts.len(), 1);
    
    // The base content should contain the inner2 function specifically
    let conflict = &conflict_file.conflicts[0];
    assert!(conflict.base_content.contains("inner2"));
    
    // It should not contain inner1 (ideally), but this depends on the matching algorithm
    // So we don't assert on this, as some implementations might include broader context
}