use std::fs::File;
use std::io::Write;
use test_conflict_parser::{parse_conflict_file_with_context_matching, ConflictFile};

#[test]
fn test_context_matching_with_explicit_sections() {
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Create base file with multiple sections
    let base_path = temp_dir.path().join("base_sections.txt");
    let base_sections_content = r#"Header information

// SECTION 1 START
function calculateTax(amount) {
    const taxRate = 0.2;
    return amount * taxRate;
}
// SECTION 1 END

// Other code in between

// SECTION 2 START
function calculateDiscount(amount) {
    const discountRate = 0.1;
    return amount * discountRate;
}
// SECTION 2 END

Footer information
"#;
    let mut base_file = File::create(&base_path).unwrap();
    base_file.write_all(base_sections_content.as_bytes()).unwrap();
    
    // Create conflict file with multiple conflicts
    let conflict_path = temp_dir.path().join("conflict_multiple.txt");
    let conflict_content = r#"Header information

<<<<<<< HEAD
function calculateTax(amount) {
    const taxRate = 0.25; // We changed the tax rate to 25%
    return amount * taxRate;
}
=======
function calculateTax(amount) {
    const taxRate = 0.2;
    // Added a minimum tax
    return Math.max(amount * taxRate, 10);
}
>>>>>>> branch-name

// Other code in between

<<<<<<< HEAD
function calculateDiscount(amount) {
    const discountRate = 0.15; // Increased discount rate
    return amount * discountRate;
}
=======
function calculateDiscount(amount, isVIP) {
    const discountRate = isVIP ? 0.2 : 0.1;
    return amount * discountRate;
}
>>>>>>> branch-name

Footer information
"#;
    let mut conflict_file = File::create(&conflict_path).unwrap();
    conflict_file.write_all(conflict_content.as_bytes()).unwrap();
    
    // Parse the conflict file with context matching
    let result = parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    assert!(result.is_ok());
    let conflict_file = result.unwrap();
    
    // Assert we found two conflicts
    assert_eq!(conflict_file.conflicts.len(), 2);
    
    // Check that correct section was matched to each conflict
    let conflict1 = &conflict_file.conflicts[0];
    let conflict2 = &conflict_file.conflicts[1];
    
    // First conflict should match section 1
    assert!(conflict1.base_content.contains("SECTION 1 START"));
    assert!(conflict1.base_content.contains("function calculateTax"));
    
    // Second conflict should match section 2
    assert!(conflict2.base_content.contains("SECTION 2 START"));
    assert!(conflict2.base_content.contains("function calculateDiscount"));
}

#[test]
fn test_context_matching_with_surrounding_context() {
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Create base file with function
    let base_path = temp_dir.path().join("base_context.txt");
    let base_context_content = r#"// User management module

// Authentication function that validates user credentials
function authenticate(username, password) {
    // Check if credentials match database records
    const user = findUserByName(username);
    if (!user) return false;
    return user.password === hashPassword(password);
}

// Authorization function that checks user permissions
function authorize(userId, resource) {
    const permissions = getUserPermissions(userId);
    return permissions.includes(resource);
}
"#;
    let mut base_file = File::create(&base_path).unwrap();
    base_file.write_all(base_context_content.as_bytes()).unwrap();
    
    // Create conflict file with surrounding context
    let conflict_path = temp_dir.path().join("conflict_context.txt");
    let conflict_content = r#"// User management module

// Authentication function that validates user credentials
<<<<<<< HEAD
function authenticate(username, password) {
    // Check if credentials match database records
    const user = findUserByName(username);
    if (!user) return false;
    // Added password expiration check
    if (user.passwordExpired) return false;
    return user.password === hashPassword(password);
}
=======
function authenticate(username, password, twoFactorCode) {
    // Check if credentials match database records
    const user = findUserByName(username);
    if (!user) return false;
    // Added two-factor authentication
    if (twoFactorCode && !validateTwoFactor(user, twoFactorCode)) return false;
    return user.password === hashPassword(password);
}
>>>>>>> branch-name

// Authorization function that checks user permissions
function authorize(userId, resource) {
    const permissions = getUserPermissions(userId);
    return permissions.includes(resource);
}
"#;
    let mut conflict_file = File::create(&conflict_path).unwrap();
    conflict_file.write_all(conflict_content.as_bytes()).unwrap();
    
    // Parse the conflict file with context matching
    let result = parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    assert!(result.is_ok());
    let conflict_file = result.unwrap();
    
    // Assert we found one conflict
    assert_eq!(conflict_file.conflicts.len(), 1);
    
    // Check that the correct section was matched to the conflict
    let conflict = &conflict_file.conflicts[0];
    
    // Should match the authenticate function
    assert!(conflict.base_content.contains("function authenticate"));
    assert!(conflict.base_content.contains("findUserByName"));
    assert!(!conflict.base_content.contains("function authorize"));
}

#[test]
fn test_fallback_to_keyword_matching() {
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Create a simple base file that will be easy to match with keywords
    let base_path = temp_dir.path().join("base_keyword.txt");
    let base_keyword_content = "function test() { return true; }\n";
    let mut base_file = File::create(&base_path).unwrap();
    base_file.write_all(base_keyword_content.as_bytes()).unwrap();
    
    // Create a simple conflict file
    let conflict_path = temp_dir.path().join("conflict_keyword.txt");
    let conflict_content = r#"<<<<<<< HEAD
function test() { return false; }
=======
function test() { return true; }
>>>>>>> branch-name
"#;
    let mut conflict_file = File::create(&conflict_path).unwrap();
    conflict_file.write_all(conflict_content.as_bytes()).unwrap();
    
    // Parse the conflict file with context matching
    let result = parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    assert!(result.is_ok());
    let conflict_file = result.unwrap();
    
    // Assert we found one conflict
    assert_eq!(conflict_file.conflicts.len(), 1);
    
    // Print the content for debugging
    let conflict = &conflict_file.conflicts[0];
    println!("Base content: '{}'\n", conflict.base_content);
    println!("Our content: '{}'\n", conflict.our_content);
    println!("Their content: '{}'\n", conflict.their_content);
    
    // Simple check that we have content at all
    assert!(!conflict.base_content.is_empty());
}