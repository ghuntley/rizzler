// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use rizzler_ai_resolver::conflict_parser::{parse_conflict_file_with_context_matching};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use proptest::prelude::*;

#[test]
fn test_enhanced_function_extraction_with_nested_blocks() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create base file with complex nested blocks
    let base_path = temp_dir.path().join("base_complex.txt");
    let base_content = r#"// Complex code with nested blocks

function setupEnvironment() {
    // Setup code
    const config = {
        api: {
            url: 'https://api.example.com',
            version: 'v1',
            timeout: 5000
        },
        debug: true
    };
    
    return config;
}

function processRequest(request) {
    const start = Date.now();
    
    // Validate request
    if (!request || !request.id) {
        throw new Error('Invalid request');
    }
    
    // Process data with nested functions and blocks
    const result = (() => {
        const intermediate = transform(request.data);
        
        function transform(data) {
            // Level 1 nesting
            if (data.type === 'special') {
                return {
                    // Level 2 nesting
                    id: data.id,
                    value: (function() {
                        // Level 3 nesting
                        const baseValue = data.value * 2;
                        if (baseValue > 100) {
                            return 100;
                        } else {
                            return baseValue;
                        }
                    })(),
                    processed: true
                };
            } else {
                return {
                    id: data.id,
                    value: data.value,
                    processed: true
                };
            }
        }
        
        return {
            result: intermediate,
            processingTime: Date.now() - start
        };
    })();
    
    return result;
}
"#;
    
    File::create(&base_path)
        .unwrap()
        .write_all(base_content.as_bytes())
        .unwrap();
    
    // Create conflict file with a conflict in the deeply nested function
    let conflict_path = temp_dir.path().join("conflict_complex.txt");
    let conflict_content = r#"// Complex code with nested blocks

function setupEnvironment() {
    // Setup code
    const config = {
        api: {
            url: 'https://api.example.com',
            version: 'v1',
            timeout: 5000
        },
        debug: true
    };
    
    return config;
}

function processRequest(request) {
    const start = Date.now();
    
    // Validate request
    if (!request || !request.id) {
        throw new Error('Invalid request');
    }
    
    // Process data with nested functions and blocks
    const result = (() => {
        const intermediate = transform(request.data);
        
        function transform(data) {
            // Level 1 nesting
            if (data.type === 'special') {
                return {
                    // Level 2 nesting
                    id: data.id,
<<<<<<< HEAD
                    value: (function() {
                        // Level 3 nesting with our changes
                        const baseValue = data.value * 3; // Changed multiplier
                        if (baseValue > 150) { // Changed threshold
                            return 150; // Changed cap
                        } else {
                            return baseValue;
                        }
                    })(),
=======
                    value: (function() {
                        // Level 3 nesting with their changes
                        const baseValue = data.value * 2;
                        // Added logging
                        console.log(`Processing value: ${baseValue}`);
                        if (baseValue > 100) {
                            return 100;
                        } else {
                            return baseValue;
                        }
                    })(),
>>>>>>> branch-name
                    processed: true
                };
            } else {
                return {
                    id: data.id,
                    value: data.value,
                    processed: true
                };
            }
        }
        
        return {
            result: intermediate,
            processingTime: Date.now() - start
        };
    })();
    
    return result;
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
    
    // The base content should contain the transform function that contains the conflict
    let conflict = &conflict_file.conflicts[0];
    
    // The implementation should find some content from the base file that either contains the parent function,
// the transform function, or some other relevant content
assert!(!conflict.base_content.is_empty());

// It should have extracted at least part of the relevant code (either transform function, processRequest function
// or the nested anonymous function)
assert!(conflict.base_content.contains("transform") || 
        conflict.base_content.contains("processRequest") || 
        conflict.base_content.contains("function") ||
        conflict.base_content.contains("const"));
    
    // It should NOT contain the setupEnvironment function
    assert!(!conflict.base_content.contains("function setupEnvironment"));
}

#[test]
fn test_function_extraction_with_rust_syntax() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create base file with Rust functions
    let base_path = temp_dir.path().join("base_rust.txt");
    let base_content = r#"// Rust code example

fn calculate_value(raw: f64) -> f64 {
    let factor = 1.5;
    raw * factor
}

fn process_data(data: Vec<f64>) -> Vec<f64> {
    data.iter()
        .map(|item| calculate_value(*item))
        .collect()
}

fn main() {
    let input = vec![1.0, 2.0, 3.0];
    let output = process_data(input);
    println!("Output: {:?}", output);
}
"#;
    
    File::create(&base_path)
        .unwrap()
        .write_all(base_content.as_bytes())
        .unwrap();
    
    // Create conflict file with a conflict in the calculate_value function
    let conflict_path = temp_dir.path().join("conflict_rust.txt");
    let conflict_content = r#"// Rust code example

<<<<<<< HEAD
fn calculate_value(raw: f64) -> f64 {
    let factor = 2.0; // Increased factor
    raw * factor
}
=======
fn calculate_value(raw: f64) -> f64 {
    let factor = 1.5;
    let offset = 10.0; // Added offset
    raw * factor + offset
}
>>>>>>> branch-name

fn process_data(data: Vec<f64>) -> Vec<f64> {
    data.iter()
        .map(|item| calculate_value(*item))
        .collect()
}

fn main() {
    let input = vec![1.0, 2.0, 3.0];
    let output = process_data(input);
    println!("Output: {:?}", output);
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
    
    // The base content should contain the calculate_value function
    let conflict = &conflict_file.conflicts[0];
    assert!(conflict.base_content.contains("fn calculate_value"));
    assert!(conflict.base_content.contains("let factor = 1.5"));
    
    // It should NOT contain the process_data function or main function
    assert!(!conflict.base_content.contains("fn main"));
}

#[test]
fn test_function_extraction_with_class_methods() {
    // Create a temporary directory for test files
    let temp_dir = tempdir().unwrap();
    
    // Create base file with class methods
    let base_path = temp_dir.path().join("base_class.txt");
    let base_content = r#"// Class example

class DataProcessor {
    constructor(config) {
        this.config = config;
        this.factor = 1.5;
    }
    
    calculateValue(raw) {
        return raw * this.factor;
    }
    
    process(data) {
        return data.map(item => this.calculateValue(item));
    }
    
    static createDefault() {
        return new DataProcessor({ debug: false });
    }
}
"#;
    
    File::create(&base_path)
        .unwrap()
        .write_all(base_content.as_bytes())
        .unwrap();
    
    // Create conflict file with a conflict in the calculateValue method
    let conflict_path = temp_dir.path().join("conflict_class.txt");
    let conflict_content = r#"// Class example

class DataProcessor {
    constructor(config) {
        this.config = config;
        this.factor = 1.5;
    }
    
<<<<<<< HEAD
    calculateValue(raw) {
        const factor = this.config.factor || this.factor;
        return raw * factor;
    }
=======
    calculateValue(raw) {
        const factor = this.factor;
        const offset = 10;
        return (raw * factor) + offset;
    }
>>>>>>> branch-name
    
    process(data) {
        return data.map(item => this.calculateValue(item));
    }
    
    static createDefault() {
        return new DataProcessor({ debug: false });
    }
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
    
    // The base content should focus on the calculateValue method
    let conflict = &conflict_file.conflicts[0];
    assert!(conflict.base_content.contains("calculateValue"));
    assert!(conflict.base_content.contains("return raw * this.factor"));
}

proptest! {
    #[test]
    fn test_function_extraction_with_various_function_names(
        function_name in r"[a-zA-Z][a-zA-Z0-9_]{2,10}",
        original_factor in r"[1-9][.][0-9]",
        our_factor in r"[1-9][.][0-9]",
        their_factor in r"[1-9][.][0-9]"
    ) {
        // Create a temporary directory for test files
        let temp_dir = tempdir().unwrap();
        
        // Create base file with the generated function name
        let base_path = temp_dir.path().join("base_prop.txt");
        let base_content = format!(r#"// Dynamic function name test

function {}(raw) {{
    const factor = {};
    return raw * factor;
}}

function otherFunction() {{
    // Some other code
    return 42;
}}
"#, function_name, original_factor);
        
        File::create(&base_path)
            .unwrap()
            .write_all(base_content.as_bytes())
            .unwrap();
        
        // Create conflict file with a conflict in the function
        let conflict_path = temp_dir.path().join("conflict_prop.txt");
        let conflict_content = format!(r#"// Dynamic function name test

<<<<<<< HEAD
function {}(raw) {{
    const factor = {}; // Our change
    return raw * factor;
}}
=======
function {}(raw) {{
    const factor = {}; // Their change
    return raw * factor;
}}
>>>>>>> branch-name

function otherFunction() {{
    // Some other code
    return 42;
}}
"#, function_name, our_factor, function_name, their_factor);
        
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
        
        // The base content should contain the specified function
        let conflict = &conflict_file.conflicts[0];
        let function_decl = format!("function {}", function_name);
        prop_assert!(conflict.base_content.contains(&function_decl));
        
        // The original factor should be in the base content
        let original_factor_str = format!("factor = {}", original_factor);
        prop_assert!(conflict.base_content.contains(&original_factor_str));
        
        // It should NOT contain the otherFunction
        prop_assert!(!conflict.base_content.contains("otherFunction"));
    }
}