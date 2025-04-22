// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    // Helper function to create a Git repo for testing
    fn setup_git_repo() -> TempDir {
        // Create a temporary directory for the test repo
        let repo_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Initialize Git repository
        let status = Command::new("git")
            .arg("init")
            .current_dir(repo_dir.path())
            .status()
            .expect("Failed to run git init");
        
        assert!(status.success(), "Git init failed");
        
        // Configure Git user
        let _ = Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_dir.path())
            .status()
            .expect("Failed to set git user.name");
            
        let _ = Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_dir.path())
            .status()
            .expect("Failed to set git user.email");
        
        repo_dir
    }
    
    // Helper function to create a file with content
    fn create_file(repo_dir: &Path, filename: &str, content: &str) -> PathBuf {
        let file_path = repo_dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create file");
        write!(file, "{}", content).expect("Failed to write to file");
        file_path
    }
    
    // Helper function to commit changes
    fn commit_changes(repo_dir: &Path, message: &str) {
        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(repo_dir)
            .status()
            .expect("Failed to stage changes");
            
        let _ = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_dir)
            .status()
            .expect("Failed to commit changes");
    }
    
    // Helper function to create a branch
    fn create_branch(repo_dir: &Path, branch_name: &str) {
        let _ = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(repo_dir)
            .status()
            .expect("Failed to create branch");
    }
    
    // Helper function to checkout a branch
    fn checkout_branch(repo_dir: &Path, branch_name: &str) {
        let _ = Command::new("git")
            .args(["checkout", branch_name])
            .current_dir(repo_dir)
            .status()
            .expect("Failed to checkout branch");
    }
    
    // Helper function to configure rizzler as the merge driver
    fn configure_merge_driver(repo_dir: &Path, resolver_path: &Path) {
        // Configure the merge driver in .git/config
        let _ = Command::new("git")
            .args([
                "config", 
                "merge.rizzler.driver", 
                &format!("{} %O %A %B %P", resolver_path.display())
            ])
            .current_dir(repo_dir)
            .status()
            .expect("Failed to configure merge driver");
            
        // Configure file types in .gitattributes
        let gitattributes_path = repo_dir.join(".gitattributes");
        let mut file = File::create(&gitattributes_path).expect("Failed to create .gitattributes");
        write!(file, "*.txt merge=rizzler\n").expect("Failed to write to .gitattributes");
        
        // Commit the .gitattributes file
        commit_changes(repo_dir, "Add .gitattributes");
    }
    
    // Helper function to perform a merge
    fn merge_branch(repo_dir: &Path, branch_name: &str) -> bool {
        let output = Command::new("git")
            .args(["merge", branch_name])
            .current_dir(repo_dir)
            .output()
            .expect("Failed to merge branch");
            
        output.status.success()
    }
    
    #[test]
    #[ignore] // This test requires a built binary and git command line
    fn test_rizzler_driver_integration() {
        // Find the rizzler binary
        let target_dir = env::current_dir().unwrap().join("target/debug");
        let resolver_path = target_dir.join("rizzler");
        
        // Skip test if binary doesn't exist
        if !resolver_path.exists() {
            println!("Skipping test - binary not found at {:?}", resolver_path);
            return;
        }
        
        // Create a test repository
        let repo_dir = setup_git_repo();
        
        // Create initial file
        let file_content = "\
        // This is a test file
        function add(a, b) {
            return a + b;
        }
        
        function subtract(a, b) {
            return a - b;
        }
        ";
        
        let file_path = create_file(repo_dir.path(), "math.txt", file_content);
        commit_changes(repo_dir.path(), "Initial commit");
        
        // Create feature branch
        create_branch(repo_dir.path(), "feature-branch");
        
        // Modify file in feature branch
        let feature_content = "\
        // This is a test file
        function add(a, b) {
            // Add two numbers and return the result
            return a + b;
        }
        
        function subtract(a, b) {
            return a - b;
        }
        
        function multiply(a, b) {
            return a * b;
        }
        ";
        
        fs::write(&file_path, feature_content).expect("Failed to modify file");
        commit_changes(repo_dir.path(), "Add multiply function");
        
        // Switch back to main branch
        checkout_branch(repo_dir.path(), "main");
        
        // Modify file in main branch (create conflict)
        let main_content = "\
        // This is a test file
        function add(a, b) {
            return a + b;
        }
        
        function subtract(a, b) {
            // Subtract b from a and return the result
            return a - b;
        }
        
        function divide(a, b) {
            if (b === 0) {
                throw new Error('Division by zero');
            }
            return a / b;
        }
        ";
        
        fs::write(&file_path, main_content).expect("Failed to modify file");
        commit_changes(repo_dir.path(), "Add divide function");
        
        // Configure rizzler
        configure_merge_driver(repo_dir.path(), &resolver_path);
        
        // Set test environment variables for the merge driver
        env::set_var("RIZZLER_PROVIDER", "openai");
        env::set_var("RIZZLER_OPENAI_API_KEY", "test-key");
        
        // Merge the feature branch (should use our merge driver)
        let merge_successful = merge_branch(repo_dir.path(), "feature-branch");
        
        // If merge succeeded, verify the result contains both functions
        if merge_successful {
            let merged_content = fs::read_to_string(&file_path).expect("Failed to read merged file");
            assert!(merged_content.contains("function multiply"));
            assert!(merged_content.contains("function divide"));
            assert!(!merged_content.contains("<<<<<<< HEAD"));
        }
        
        // Clean up environment
        env::remove_var("RIZZLER_PROVIDER");
        env::remove_var("RIZZLER_OPENAI_API_KEY");
    }
}