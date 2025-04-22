// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// Helper function to create a temporary Git repository
fn create_temp_git_repo() -> io::Result<TempDir> {
    let temp_dir = tempfile::tempdir()?;
    
    // Initialize Git repo
    let status = Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .status()?;
    
    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to initialize Git repository"));
    }
    
    // Configure Git user (needed for commits)
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .status()?;
    
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .status()?;
    
    Ok(temp_dir)
}

// Helper function to check if a pattern exists in a file
fn file_contains(path: &Path, pattern: &str) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    Ok(contents.contains(pattern))
}

#[test]
#[ignore = "Requires Git and executable binary"]
fn test_setup_command_local() {
    // Create a temporary Git repository
    let temp_dir = create_temp_git_repo().expect("Failed to create temporary Git repository");
    
    // Path to the git-merge-ai-resolver binary
    // In a real test, you'd use the actual binary path
    let binary_path = env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .join("git-merge-ai-resolver");
    
    // Run setup command
    let status = Command::new(&binary_path)
        .args(["setup", "--local", "--extensions", "js", "py", "rs"])
        .current_dir(&temp_dir)
        .status()
        .expect("Failed to execute command");
    
    assert!(status.success(), "Setup command failed");
    
    // Check if .git/config was updated correctly
    let git_config_path = temp_dir.path().join(".git/config");
    let contains_merge_driver = file_contains(&git_config_path, "[merge \"git-merge-ai-resolver\"]").unwrap();
    assert!(contains_merge_driver, ".git/config does not contain merge driver configuration");
    
    // Check if .gitattributes was created and contains file extensions
    let gitattributes_path = temp_dir.path().join(".gitattributes");
    assert!(gitattributes_path.exists(), ".gitattributes file was not created");
    
    let contains_js = file_contains(&gitattributes_path, "*.js merge=git-merge-ai-resolver").unwrap();
    let contains_py = file_contains(&gitattributes_path, "*.py merge=git-merge-ai-resolver").unwrap();
    let contains_rs = file_contains(&gitattributes_path, "*.rs merge=git-merge-ai-resolver").unwrap();
    
    assert!(contains_js, ".gitattributes does not contain JS configuration");
    assert!(contains_py, ".gitattributes does not contain PY configuration");
    assert!(contains_rs, ".gitattributes does not contain RS configuration");
}

#[test]
#[ignore = "Requires Git and executable binary"]
fn test_setup_command_global() {
    // This test would be similar but checking global Git config
    // For simplicity in automated tests, we'll skip actual global config modifications
    // and just check that the command structure works
    
    // Create a temporary directory (not a Git repo, just for executing the command)
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    
    // Path to the git-merge-ai-resolver binary
    let binary_path = env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .join("git-merge-ai-resolver");
    
    // Run setup command with --dry-run to avoid actually modifying global config
    // Note: --dry-run would need to be implemented in the actual command
    let output = Command::new(&binary_path)
        .args(["setup", "--global", "--extensions", "js", "py", "rs", "--dry-run"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Check that the command execution includes expected text
    // This depends on how you implement the output of --dry-run
    assert!(stdout.contains("global"), "Setup command output does not mention global config");
}