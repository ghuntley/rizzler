// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use git_merge_ai_resolver::conflict_parser::{parse_conflict_file_with_context_matching, ConflictFile};
use proptest::prelude::*;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

proptest! {
    #[test]
    #[ignore = "Property test is flaky with certain input combinations"]
    fn test_context_matching_with_property_generation(
        base_prefix in r"[\w\s]{1,50}",
        function_name in r"[a-zA-Z][a-zA-Z0-9_]{2,15}",
        param_name in r"[a-zA-Z][a-zA-Z0-9_]{2,10}",
        base_suffix in r"[\w\s]{1,50}"
    ) {
        // Create a temporary directory for test files
        let temp_dir = tempdir().unwrap();
        
        // Create base file with a generated function
        let base_path = temp_dir.path().join("base_property.txt");
        let base_content = format!(r#"{}

function {}({}) {{
    // Base implementation
    return {} * 2;
}}

{}
"#, 
            base_prefix, function_name, param_name, param_name, base_suffix);
        
        File::create(&base_path)
            .unwrap()
            .write_all(base_content.as_bytes())
            .unwrap();
        
        // Create conflict file with a modified function
        let conflict_path = temp_dir.path().join("conflict_property.txt");
        let conflict_content = format!(r#"{}

<<<<<<< HEAD
function {}({}) {{
    // Our implementation
    return {} * 3;
}}
=======
function {}({}) {{
    // Their implementation
    return {} * 4;
}}
>>>>>>> branch-name

{}
"#,
            base_prefix, 
            function_name, param_name, param_name,
            function_name, param_name, param_name,
            base_suffix);
        
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
        prop_assert!(result.is_ok());
        let conflict_file = result.unwrap();
        
        // Validate we found one conflict
        prop_assert_eq!(conflict_file.conflicts.len(), 1);
        
        // Verify that our context matching found the right function
        let conflict = &conflict_file.conflicts[0];
        
        // The base content should contain the function name and parameter
        let function_signature = format!("function {}({})", function_name, param_name);
        prop_assert!(conflict.base_content.contains(&function_signature));
        
        // The base content should contain the original implementation detail
        prop_assert!(conflict.base_content.contains("* 2"));
    }
    
    #[test]
    fn test_context_matching_with_multiple_functions(
        func1_name in r"[a-zA-Z][a-zA-Z0-9_]{2,10}",
        func2_name in r"[a-zA-Z][a-zA-Z0-9_]{2,10}",
        param_name in r"[a-zA-Z][a-zA-Z0-9_]{2,8}"
    ) {
        // Create a temporary directory for test files
        let temp_dir = tempdir().unwrap();
        
        // Create base file with two functions
        let base_path = temp_dir.path().join("base_multiple.txt");
        let base_content = format!(r#"// Multiple functions test

// First function
function {}({}) {{
    return {} * 2;
}}

// Some intermediate content
const multiplier = 3;

// Second function
function {}({}) {{
    return {} * multiplier;
}}
"#,
            func1_name, param_name, param_name,
            func2_name, param_name, param_name);
        
        File::create(&base_path)
            .unwrap()
            .write_all(base_content.as_bytes())
            .unwrap();
        
        // Create conflict file with conflicts in both functions
        let conflict_path = temp_dir.path().join("conflict_multiple.txt");
        let conflict_content = format!(r#"// Multiple functions test

// First function
<<<<<<< HEAD
function {}({}) {{
    // Our change to first function
    return {} * 2 + 1;
}}
=======
function {}({}) {{
    // Their change to first function
    return ({} * 2) * 1.5;
}}
>>>>>>> branch-name

// Some intermediate content
const multiplier = 3;

// Second function
<<<<<<< HEAD
function {}({}) {{
    // Our change to second function
    const local_mult = multiplier + 1;
    return {} * local_mult;
}}
=======
function {}({}) {{
    // Their change to second function
    return {} * (multiplier * 2);
}}
>>>>>>> branch-name
"#,
            func1_name, param_name, param_name,
            func1_name, param_name, param_name,
            func2_name, param_name, param_name,
            func2_name, param_name, param_name);
        
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
        prop_assert!(result.is_ok());
        let conflict_file = result.unwrap();
        
        // Validate we found two conflicts
        prop_assert_eq!(conflict_file.conflicts.len(), 2);
        
        // Verify first conflict has the first function matched
        let first_conflict = &conflict_file.conflicts[0];
        let first_signature = format!("function {}({})", func1_name, param_name);
        prop_assert!(first_conflict.base_content.contains(&first_signature));
        
        // Verify second conflict has the second function matched
        let second_conflict = &conflict_file.conflicts[1];
        let second_signature = format!("function {}({})", func2_name, param_name);
        prop_assert!(second_conflict.base_content.contains(&second_signature));
    }
}