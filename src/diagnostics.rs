// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::AIProvider;
use crate::providers::{OpenAIProvider, ClaudeProvider, GeminiProvider, BedrockProvider};
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

/// Represents the result of a diagnostic check
pub struct DiagnosticResult {
    pub name: String,
    pub status: DiagnosticStatus,
    pub message: String,
    pub resolution: Option<String>,
}

/// Status of a diagnostic check
#[derive(PartialEq)]
pub enum DiagnosticStatus {
    Pass,
    Warning,
    Fail,
}

impl DiagnosticStatus {
    pub fn as_str(&self) -> &str {
        match self {
            DiagnosticStatus::Pass => "PASS",
            DiagnosticStatus::Warning => "WARN",
            DiagnosticStatus::Fail => "FAIL",
        }
    }
}

/// Run all diagnostics and return results
pub fn run_diagnostics() -> Vec<DiagnosticResult> {
    let mut results = Vec::new();
    
    // Check Git installation
    results.push(check_git_installation());
    
    // Check Git configuration
    results.push(check_git_configuration());
    
    // Check gitattributes configuration
    results.push(check_gitattributes());
    
    // Check if any AI providers are available
    results.push(check_ai_providers());
    
    // Check environment variables
    results.push(check_environment_variables());
    
    results
}

/// Check if Git is installed and accessible
fn check_git_installation() -> DiagnosticResult {
    let output = Command::new("git").arg("--version").output();
    
    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            DiagnosticResult {
                name: "Git Installation".to_string(),
                status: DiagnosticStatus::Pass,
                message: format!("Git is installed: {}", version.trim()),
                resolution: None,
            }
        },
        Ok(_) => {
            DiagnosticResult {
                name: "Git Installation".to_string(),
                status: DiagnosticStatus::Fail,
                message: "Git is installed but the version command failed".to_string(),
                resolution: Some("Ensure Git is properly installed and accessible in your PATH".to_string()),
            }
        },
        Err(_) => {
            DiagnosticResult {
                name: "Git Installation".to_string(),
                status: DiagnosticStatus::Fail,
                message: "Git is not installed or not in PATH".to_string(),
                resolution: Some("Install Git and ensure it's in your PATH".to_string()),
            }
        },
    }
}

/// Check if Git is configured with our merge driver
fn check_git_configuration() -> DiagnosticResult {
    // Check global configuration
    let global_config = Command::new("git")
        .args(["config", "--global", "--get", "merge.rizzler.driver"])
        .output();
        
    // Check local configuration
    let local_config = Command::new("git")
        .args(["config", "--local", "--get", "merge.rizzler.driver"])
        .output();
    
    match (global_config, local_config) {
        (Ok(global), Ok(local)) if global.status.success() || local.status.success() => {
            DiagnosticResult {
                name: "Git Merge Driver Configuration".to_string(),
                status: DiagnosticStatus::Pass,
                message: "Git is configured with rizzler merge driver".to_string(),
                resolution: None,
            }
        },
        _ => {
            DiagnosticResult {
                name: "Git Merge Driver Configuration".to_string(),
                status: DiagnosticStatus::Warning,
                message: "Git is not configured with rizzler merge driver".to_string(),
                resolution: Some("Run 'rizzler setup --global --extensions js py rs' to configure".to_string()),
            }
        },
    }
}

/// Check if gitattributes is configured with our merge driver
fn check_gitattributes() -> DiagnosticResult {
    // Check global gitattributes
    let home_dir = dirs::home_dir();
    let global_gitattributes = home_dir.map(|dir| dir.join(".gitattributes"));
    
    // Check local gitattributes
    let local_gitattributes = Path::new(".gitattributes");
    
    let global_exists = global_gitattributes.as_ref().map_or(false, |path| path.exists());
    let local_exists = local_gitattributes.exists();
    
    let global_configured = if global_exists {
        let global_path = global_gitattributes.as_ref().unwrap();
        check_gitattributes_file(global_path)
    } else {
        false
    };
    
    let local_configured = if local_exists {
        check_gitattributes_file(local_gitattributes)
    } else {
        false
    };
    
    if global_configured || local_configured {
        DiagnosticResult {
            name: "Gitattributes Configuration".to_string(),
            status: DiagnosticStatus::Pass,
            message: "Gitattributes is configured with rizzler".to_string(),
            resolution: None,
        }
    } else if global_exists || local_exists {
        DiagnosticResult {
            name: "Gitattributes Configuration".to_string(),
            status: DiagnosticStatus::Warning,
            message: "Gitattributes file exists but does not configure rizzler".to_string(),
            resolution: Some("Run 'rizzler setup' to configure gitattributes".to_string()),
        }
    } else {
        DiagnosticResult {
            name: "Gitattributes Configuration".to_string(),
            status: DiagnosticStatus::Warning,
            message: "No gitattributes file found".to_string(),
            resolution: Some("Run 'rizzler setup' to create and configure gitattributes".to_string()),
        }
    }
}

/// Check if a gitattributes file contains our merge driver
fn check_gitattributes_file(path: &Path) -> bool {
    if let Ok(contents) = std::fs::read_to_string(path) {
        contents.contains("merge=rizzler")
    } else {
        false
    }
}

/// Check if any AI providers are available
fn check_ai_providers() -> DiagnosticResult {
    let mut available_providers = Vec::new();
    let mut unavailable_providers = Vec::new();
    
    // Check OpenAI
    match OpenAIProvider::new() {
        Ok(provider) if provider.is_available() => {
            available_providers.push("OpenAI");
        },
        _ => {
            unavailable_providers.push("OpenAI");
        }
    }
    
    // Check Claude
    match ClaudeProvider::new() {
        Ok(provider) if provider.is_available() => {
            available_providers.push("Claude");
        },
        _ => {
            unavailable_providers.push("Claude");
        }
    }
    
    // Check Gemini
    match GeminiProvider::new() {
        Ok(provider) if provider.is_available() => {
            available_providers.push("Gemini");
        },
        _ => {
            unavailable_providers.push("Gemini");
        }
    }
    
    // Check Bedrock
    match BedrockProvider::new() {
        Ok(provider) if provider.is_available() => {
            available_providers.push("AWS Bedrock");
        },
        _ => {
            unavailable_providers.push("AWS Bedrock");
        }
    }
    
    if !available_providers.is_empty() {
        DiagnosticResult {
            name: "AI Providers".to_string(),
            status: DiagnosticStatus::Pass,
            message: format!("Available providers: {}", available_providers.join(", ")),
            resolution: if !unavailable_providers.is_empty() {
                Some(format!("Unavailable providers: {}", unavailable_providers.join(", ")))
            } else {
                None
            },
        }
    } else {
        DiagnosticResult {
            name: "AI Providers".to_string(),
            status: DiagnosticStatus::Fail,
            message: "No AI providers are available".to_string(),
            resolution: Some("Configure at least one AI provider by setting the appropriate environment variables (RIZZLER_OPENAI_API_KEY, RIZZLER_CLAUDE_API_KEY, etc.)".to_string()),
        }
    }
}

/// Check if required environment variables are set
fn check_environment_variables() -> DiagnosticResult {
    let mut set_variables = Vec::new();
    let important_variables = [
        "RIZZLER_OPENAI_API_KEY",
        "RIZZLER_CLAUDE_API_KEY",
        "RIZZLER_GEMINI_API_KEY",
        "AWS_ACCESS_KEY_ID", // For Bedrock
        "RIZZLER_SYSTEM_PROMPT",
        "RIZZLER_TIMEOUT",
    ];
    
    for var in important_variables.iter() {
        if env::var(var).is_ok() {
            set_variables.push(*var);
        }
    }
    
    if !set_variables.is_empty() {
        DiagnosticResult {
            name: "Environment Variables".to_string(),
            status: DiagnosticStatus::Pass,
            message: format!("Set variables: {}", set_variables.join(", ")),
            resolution: None,
        }
    } else {
        DiagnosticResult {
            name: "Environment Variables".to_string(),
            status: DiagnosticStatus::Warning,
            message: "No configuration environment variables are set".to_string(),
            resolution: Some("Set at least one API key environment variable to enable AI resolution".to_string()),
        }
    }
}

/// Format diagnostic results for display
pub fn format_diagnostic_results(results: &[DiagnosticResult]) -> String {
    let mut output = String::new();
    output.push_str("Diagnostic Results:\n");
    output.push_str("==================\n\n");
    
    for result in results {
        output.push_str(&format!("[{}] {}\n", result.status.as_str(), result.name));
        output.push_str(&format!("     {}\n", result.message));
        
        if let Some(resolution) = &result.resolution {
            output.push_str(&format!("     FIX: {}\n", resolution));
        }
        
        output.push_str("\n");
    }
    
    // Add summary
    let pass_count = results.iter().filter(|r| r.status == DiagnosticStatus::Pass).count();
    let warn_count = results.iter().filter(|r| r.status == DiagnosticStatus::Warning).count();
    let fail_count = results.iter().filter(|r| r.status == DiagnosticStatus::Fail).count();
    
    output.push_str(&format!("Summary: {} passed, {} warnings, {} failed\n", pass_count, warn_count, fail_count));
    
    output
}

/// Write diagnostic results to a file
pub fn write_diagnostic_results(results: &[DiagnosticResult], path: Option<&str>) -> io::Result<()> {
    let formatted = format_diagnostic_results(results);
    
    match path {
        Some(path) => {
            let mut file = File::create(path)?;
            file.write_all(formatted.as_bytes())?;
            Ok(())
        },
        None => Ok(()),
    }
}