// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use tracing::{debug, error, info, warn};

/// Error type for Git setup operations
#[derive(Debug)]
pub enum SetupError {
    /// IO error
    Io(io::Error),
    
    /// Git command failed
    GitCommandFailed(String),
    
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::GitCommandFailed(msg) => write!(f, "Git command failed: {}", msg),
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for SetupError {}

impl From<io::Error> for SetupError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Configure git-merge-ai-resolver as a Git merge driver
///
/// # Arguments
///
/// * `global` - If true, configure globally in user's .gitconfig
/// * `local` - If true, configure only for current repository
/// * `extensions` - File extensions to associate with the merge driver
/// * `dry_run` - If true, don't actually modify any files (just print what would happen)
pub fn setup_git_integration(
    global: bool,
    local: bool,
    extensions: &[String],
    dry_run: bool,
) -> Result<(), SetupError> {
    // Validate args
    if !global && !local {
        return Err(SetupError::InvalidConfig(
            "Either --global or --local must be specified".to_string(),
        ));
    }
    
    if global && local {
        return Err(SetupError::InvalidConfig(
            "Only one of --global or --local can be specified".to_string(),
        ));
    }
    
    if extensions.is_empty() {
        return Err(SetupError::InvalidConfig(
            "At least one file extension must be specified".to_string(),
        ));
    }
    
    // Check if git is available
    match Command::new("git").arg("--version").output() {
        Ok(_) => {
            info!("Git detected");
        }
        Err(err) => {
            error!("Git not found: {}", err);
            return Err(SetupError::GitCommandFailed(format!("Git not found: {}", err)));
        }
    }
    
    // Configure git merge driver
    configure_git_merge_driver(global, dry_run)?;
    
    // Configure gitattributes
    configure_gitattributes(global, local, extensions, dry_run)?;
    
    Ok(())
}

/// Configure Git merge driver in .gitconfig
fn configure_git_merge_driver(global: bool, dry_run: bool) -> Result<(), SetupError> {
    // Extract the path to the git-merge-ai-resolver binary
    let binary_path = std::env::current_exe()?
        .to_string_lossy()
        .to_string();
    
    let args = if global {
        vec![
            "config".to_string(),
            "--global".to_string(),
            "merge.git-merge-ai-resolver.name".to_string(),
            "AI-powered Git merge conflict resolver".to_string(),
        ]
    } else {
        vec![
            "config".to_string(),
            "merge.git-merge-ai-resolver.name".to_string(),
            "AI-powered Git merge conflict resolver".to_string(),
        ]
    };
    
    if dry_run {
        info!("Would run: git {}", args.join(" "));
    } else {
        let status = Command::new("git").args(&args).status()?;
        
        if !status.success() {
            return Err(SetupError::GitCommandFailed(
                "Failed to configure merge driver name".to_string(),
            ));
        }
    }
    
    // Configure merge driver
    let driver_args = if global {
        vec![
            "config".to_string(),
            "--global".to_string(),
            "merge.git-merge-ai-resolver.driver".to_string(),
            format!("{} %O %A %B %P", binary_path),
        ]
    } else {
        vec![
            "config".to_string(),
            "merge.git-merge-ai-resolver.driver".to_string(),
            format!("{} %O %A %B %P", binary_path),
        ]
    };
    
    if dry_run {
        info!("Would run: git {}", driver_args.join(" "));
    } else {
        let status = Command::new("git").args(&driver_args).status()?;
        
        if !status.success() {
            return Err(SetupError::GitCommandFailed(
                "Failed to configure merge driver".to_string(),
            ));
        }
    }
    
    // Configure trustExitCode
    let trust_args = if global {
        vec![
            "config".to_string(),
            "--global".to_string(),
            "merge.git-merge-ai-resolver.trustExitCode".to_string(),
            "true".to_string(),
        ]
    } else {
        vec![
            "config".to_string(),
            "merge.git-merge-ai-resolver.trustExitCode".to_string(),
            "true".to_string(),
        ]
    };
    
    if dry_run {
        info!("Would run: git {}", trust_args.join(" "));
    } else {
        let status = Command::new("git").args(&trust_args).status()?;
        
        if !status.success() {
            return Err(SetupError::GitCommandFailed(
                "Failed to configure trustExitCode".to_string(),
            ));
        }
    }
    
    Ok(())
}

/// Configure gitattributes for file extensions
fn configure_gitattributes(
    global: bool,
    local: bool,
    extensions: &[String],
    dry_run: bool,
) -> Result<(), SetupError> {
    let gitattributes_path = if global {
        // Global gitattributes is typically in user's home directory
        let home_dir = dirs::home_dir().ok_or_else(|| {
            SetupError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "Home directory not found",
            ))
        })?;
        
        home_dir.join(".gitattributes")
    } else if local {
        // Local gitattributes is in the current directory
        Path::new(".gitattributes").to_path_buf()
    } else {
        // This shouldn't happen due to validation above
        return Err(SetupError::InvalidConfig(
            "Neither global nor local specified".to_string(),
        ));
    };
    
    if dry_run {
        info!("Would update gitattributes at: {}", gitattributes_path.display());
        for ext in extensions {
            info!("Would add: *.{} merge=git-merge-ai-resolver", ext);
        }
        return Ok(());
    }
    
    // Check if the file exists, create it if it doesn't
    let file_exists = gitattributes_path.exists();
    
    let mut file = if file_exists {
        // Open in append mode if the file exists
        OpenOptions::new()
            .write(true)
            .append(true)
            .open(&gitattributes_path)?
    } else {
        // Create the file if it doesn't exist
        File::create(&gitattributes_path)?
    };
    
    // Add a header if the file is new
    if !file_exists {
        writeln!(
            file,
            "# gitattributes configuration for git-merge-ai-resolver"
        )?;
        writeln!(file, "# Generated automatically by git-merge-ai-resolver setup")?;
        writeln!(file)?;
    } else {
        // Add a blank line if the file exists
        writeln!(file)?;
        writeln!(file, "# Additional configuration from git-merge-ai-resolver")?;
    }
    
    // Write the configuration for each extension
    for ext in extensions {
        writeln!(file, "*.{} merge=git-merge-ai-resolver", ext)?;
    }
    
    Ok(())
}