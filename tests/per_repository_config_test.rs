// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use rizzler::config::{Config, ConfigError};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[test]
#[ignore]
fn test_per_repository_config_loading() {
    // Create a temporary directory to simulate a Git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Simulate a repository-specific configuration file
    let config_path = repo_path.join(".rizzler");
    let mut file = File::create(&config_path).unwrap();
    
    // Write a TOML configuration file
    writeln!(
        file,
        r#"
        [ai_provider]
        default_provider = "claude"
        default_model = "claude-3-opus"
        system_prompt = "Repository-specific prompt"
        timeout_seconds = 60
        
        [resolution]
        default_strategy = "ai-windowing"
        
        [resolution.extension_strategies]
        js = "ai-fallback"
        rs = "ai-windowing"
        md = "simple"
        "#
    )
    .unwrap();
    
    // Set the current working directory to the temp dir for the test
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(repo_path).unwrap();
    
    // Create a configuration object and load from the repository-specific file
    let config = Config::load_with_repository_config().unwrap();
    
    // Verify the configuration was loaded from the file
    assert_eq!(config.ai_provider.default_provider, Some("claude".to_string()));
    assert_eq!(config.ai_provider.default_model, Some("claude-3-opus".to_string()));
    assert_eq!(config.ai_provider.system_prompt, Some("Repository-specific prompt".to_string()));
    assert_eq!(config.ai_provider.timeout_seconds, 60);
    assert_eq!(config.resolution.default_strategy, "ai-windowing");
    assert_eq!(config.resolution.extension_strategies.get("js"), Some(&"ai-fallback".to_string()));
    assert_eq!(config.resolution.extension_strategies.get("rs"), Some(&"ai-windowing".to_string()));
    assert_eq!(config.resolution.extension_strategies.get("md"), Some(&"simple".to_string()));
    
    // Reset the current working directory
    env::set_current_dir(original_dir).unwrap();
}

#[test]
#[ignore]
fn test_repository_config_precedence() {
    // Create a temporary directory to simulate a Git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Simulate a repository-specific configuration file
    let config_path = repo_path.join(".rizzler");
    let mut file = File::create(&config_path).unwrap();
    
    writeln!(
        file,
        r#"
        [ai_provider]
        default_provider = "claude"
        default_model = "claude-3-opus"
        
        [resolution]
        default_strategy = "ai-windowing"
        "#
    )
    .unwrap();
    
    // Set environment variables that should override the file-based config
    env::set_var("RIZZLER_PROVIDER_DEFAULT", "openai");
    
    // Set the current working directory to the temp dir for the test
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(repo_path).unwrap();
    
    // Create a configuration object and load from both sources
    let config = Config::load_with_repository_config().unwrap();
    
    // Verify that environment variables take precedence over repository-specific config
    assert_eq!(config.ai_provider.default_provider, Some("openai".to_string()));
    
    // But the model should still be from the repository-specific config
    assert_eq!(config.ai_provider.default_model, Some("claude-3-opus".to_string()));
    assert_eq!(config.resolution.default_strategy, "ai-windowing");
    
    // Clean up environment
    env::remove_var("RIZZLER_PROVIDER_DEFAULT");
    
    // Reset the current working directory
    env::set_current_dir(original_dir).unwrap();
}

#[test]
#[ignore]
fn test_save_repository_config() {
    // Create a temporary directory to simulate a Git repository
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Set the current working directory to the temp dir for the test
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(repo_path).unwrap();
    
    // Create a configuration object
    let mut config = Config::default();
    config.ai_provider.default_provider = Some("gemini".to_string());
    config.ai_provider.default_model = Some("gemini-pro".to_string());
    config.resolution.default_strategy = "ai-fallback".to_string();
    config.resolution.extension_strategies.insert("py".to_string(), "ai-windowing".to_string());
    
    // Save the configuration to the repository
    let result = config.save_to_repository();
    assert!(result.is_ok());
    
    // Verify that the file was created
    let config_path = repo_path.join(".rizzler");
    assert!(config_path.exists());
    
    // Load the configuration from the file and verify it matches
    let loaded_config = Config::load_with_repository_config().unwrap();
    assert_eq!(loaded_config.ai_provider.default_provider, Some("gemini".to_string()));
    assert_eq!(loaded_config.ai_provider.default_model, Some("gemini-pro".to_string()));
    assert_eq!(loaded_config.resolution.default_strategy, "ai-fallback");
    assert_eq!(loaded_config.resolution.extension_strategies.get("py"), Some(&"ai-windowing".to_string()));
    
    // Reset the current working directory
    env::set_current_dir(original_dir).unwrap();
}