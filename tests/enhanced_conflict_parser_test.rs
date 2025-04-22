use rizzler_ai_resolver::conflict_parser::{parse_conflict_file, parse_conflict_file_with_base, ConflictParseError};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_enhanced_conflict_parser_with_matching_sections() {
    // Create temporary files for testing
    let temp_dir = tempdir().unwrap();
    
    // Create base file with content that has identifiable sections
    let base_path = temp_dir.path().join("base.txt");
    let base_content = "
Before section 1
SECTION 1 START
This is common content in section 1.
This is base-specific content in section 1.
SECTION 1 END
Between sections
SECTION 2 START
This is common content in section 2.
This is base-specific content in section 2.
SECTION 2 END
After section 2
";
    let mut base_file = File::create(&base_path).unwrap();
    base_file.write_all(base_content.as_bytes()).unwrap();
    
    // Create conflict file with sections that match the base file
    let conflict_path = temp_dir.path().join("conflict.txt");
    let conflict_content = "
Before section 1
SECTION 1 START
This is common content in section 1.
<<<<<<< HEAD
This is our-specific content in section 1.
=======
This is their-specific content in section 1.
>>>>>>> branch-name
SECTION 1 END
Between sections
SECTION 2 START
This is common content in section 2.
<<<<<<< HEAD
This is our-specific content in section 2.
=======
This is their-specific content in section 2.
>>>>>>> branch-name
SECTION 2 END
After section 2
";
    let mut conflict_file = File::create(&conflict_path).unwrap();
    conflict_file.write_all(conflict_content.as_bytes()).unwrap();
    
    // Parse the conflict file with smart base content matching
    let result = rizzler_ai_resolver::conflict_parser::parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    assert!(result.is_ok());
    
    let conflict_file = result.unwrap();
    assert_eq!(conflict_file.conflicts.len(), 2);
    
    // Check that the base content for each conflict contains only the relevant section
    let conflict1 = &conflict_file.conflicts[0];
    assert!(conflict1.base_content.contains("This is base-specific content in section 1."));
    assert!(!conflict1.base_content.contains("This is base-specific content in section 2."));
    
    let conflict2 = &conflict_file.conflicts[1];
    assert!(conflict2.base_content.contains("This is base-specific content in section 2."));
    assert!(!conflict2.base_content.contains("This is base-specific content in section 1."));
}

#[test]
fn test_enhanced_conflict_parser_with_non_matching_sections() {
    // Test case where exact section matching fails and we fall back to approximate matching
    let temp_dir = tempdir().unwrap();
    
    // Create base file with content
    let base_path = temp_dir.path().join("base.txt");
    let base_content = "
First part of the document.
Here's some base content about topic A.
Here's more base content about topic B.
Last part of the document.
";
    let mut base_file = File::create(&base_path).unwrap();
    base_file.write_all(base_content.as_bytes()).unwrap();
    
    // Create conflict file with sections that don't exactly match the base
    let conflict_path = temp_dir.path().join("conflict.txt");
    let conflict_content = "
First part of the document.
<<<<<<< HEAD
Here's our modified content about topic A.
=======
Here's their modified content about topic A.
>>>>>>> branch-name
Here's more content about topic B.
<<<<<<< HEAD
Our changes to topic B.
=======
Their changes to topic B.
>>>>>>> branch-name
Last part of the document.
";
    let mut conflict_file = File::create(&conflict_path).unwrap();
    conflict_file.write_all(conflict_content.as_bytes()).unwrap();
    
    // Parse the conflict file with smart base content matching
    let result = rizzler_ai_resolver::conflict_parser::parse_conflict_file_with_context_matching(
        conflict_path.to_str().unwrap(),
        base_path.to_str().unwrap()
    );
    
    assert!(result.is_ok());
    
    let conflict_file = result.unwrap();
    assert_eq!(conflict_file.conflicts.len(), 2);
    
    // In this case, since exact matching is difficult, we expect at least some content
    // from the base file in each conflict region
    assert!(!conflict_file.conflicts[0].base_content.is_empty());
    assert!(!conflict_file.conflicts[1].base_content.is_empty());
    
    // The first conflict should get a base content related to topic A
    assert!(conflict_file.conflicts[0].base_content.contains("topic A"));
    
    // The second conflict should get a base content related to topic B
    assert!(conflict_file.conflicts[1].base_content.contains("topic B"));
}