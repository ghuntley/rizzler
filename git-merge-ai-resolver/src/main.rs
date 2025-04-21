// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use clap::{Parser, Subcommand};
use git_merge_ai_resolver::{Config, conflict_parser, git_integration};
use std::env;
use std::process;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(name = "git-merge-ai-resolver")]
#[command(about = "AI-powered Git merge conflict resolver")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure git-merge-ai-resolver as a merge driver in Git
    Setup(SetupArgs),
    
    /// View and modify configuration settings
    Config(ConfigArgs),
    
    /// Manually resolve conflicts in a file
    Resolve(ResolveArgs),
    
    /// Display version information
    Version,
    
    /// Check configuration and diagnose issues
    Doctor(DoctorArgs),
}

#[derive(Parser)]
struct SetupArgs {
    /// Configure globally in user's .gitconfig
    #[arg(long)]
    global: bool,
    
    /// Configure only for current repository
    #[arg(long)]
    local: bool,
    
    /// Specify file extensions to associate with the merge driver
    #[arg(long = "extensions")]
    extensions: Vec<String>,
}

#[derive(Parser)]
struct ConfigArgs {
    /// Configuration subcommand
    #[command(subcommand)]
    action: Option<ConfigActions>,
}

#[derive(Subcommand)]
enum ConfigActions {
    /// Get the value of a specific configuration key
    Get { key: String },
    
    /// Set a configuration value
    Set { key: String, value: String },
    
    /// List all configuration values
    List,
}

#[derive(Parser)]
struct ResolveArgs {
    /// Path to file with conflicts
    file: String,
    
    /// Specify output file (default: stdout)
    #[arg(long)]
    output: Option<String>,
    
    /// Specify AI provider to use
    #[arg(long)]
    provider: Option<String>,
    
    /// Specify model to use
    #[arg(long)]
    model: Option<String>,
    
    /// Resolution strategy (ai, rule-based, manual)
    #[arg(long)]
    strategy: Option<String>,
}

#[derive(Parser)]
struct DoctorArgs {
    /// Check specific component
    #[arg(long)]
    component: Option<String>,
}

fn setup_logging() {
    let filter = EnvFilter::try_from_env("GIT_MERGE_LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    fmt()
        .with_env_filter(filter)
        .init();
}

fn main() {
    // Initialize logging
    setup_logging();
    
    // Load configuration
    let config = match Config::load() {
        Ok(config) => config,
        Err(err) => {
            error!("Failed to load configuration: {}", err);
            process::exit(1);
        }
    };
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    match &cli.command {
        Some(Commands::Setup(args)) => {
            info!("Setting up git-merge-ai-resolver as a Git merge driver");
            // TODO: Implement setup functionality
            println!("Setting up git-merge-ai-resolver as a Git merge driver");
            if args.global {
                println!("Configuring globally");
            } else if args.local {
                println!("Configuring locally");
            }
            println!("File extensions: {:?}", args.extensions);
        }
        Some(Commands::Config(args)) => {
            match &args.action {
                Some(ConfigActions::Get { key }) => {
                    info!("Getting config value for key: {}", key);
                    match config.get(key) {
                        Some(value) => println!("{}", value),
                        None => {
                            eprintln!("Config key not found: {}", key);
                            process::exit(1);
                        }
                    }
                }
                Some(ConfigActions::Set { key, value }) => {
                    info!("Setting config value for key: {} to value: {}", key, value);
                    // TODO: Implement config set with saving
                    println!("Setting {} = {}", key, value);
                }
                Some(ConfigActions::List) => {
                    info!("Listing all configuration values");
                    // TODO: Implement proper config listing
                    println!("Configuration values will be listed here");
                }
                None => {
                    println!("Use 'config get', 'config set', or 'config list'");
                }
            }
        }
        Some(Commands::Resolve(args)) => {
            info!("Resolving conflicts in file: {}", args.file);
            
            // Parse conflict file
            let conflict_file = match conflict_parser::parse_conflict_file(&args.file) {
                Ok(file) => file,
                Err(err) => {
                    error!("Failed to parse conflict file: {}", err);
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            };
            
            println!("Found {} conflicts in file", conflict_file.conflicts.len());
            
            // Create resolution engine
            let engine = git_merge_ai_resolver::resolution_engine::ResolutionEngine::new();
            
            // Resolve conflicts
            let resolution_result = match args.strategy.as_deref() {
                Some(strategy) => {
                    match engine.resolve_with_strategy(&conflict_file, strategy) {
                        Ok(result) => result,
                        Err(err) => {
                            error!("Failed to resolve conflicts with strategy {}: {}", strategy, err);
                            eprintln!("Error: {}", err);
                            process::exit(1);
                        }
                    }
                },
                None => {
                    match engine.resolve_file(&conflict_file) {
                        Ok(result) => result,
                        Err(err) => {
                            error!("Failed to resolve conflicts: {}", err);
                            eprintln!("Error: {}", err);
                            process::exit(1);
                        }
                    }
                }
            };
            
            // Write the result
            let output_path = args.output.as_deref().unwrap_or(&args.file);
            match engine.write_resolution(&resolution_result, Some(output_path)) {
                Ok(_) => {
                    println!(
                        "Resolved {}/{} conflicts using strategy '{}'", 
                        resolution_result.resolved_count,
                        resolution_result.resolved_count + resolution_result.unresolved_count,
                        resolution_result.strategy_name
                    );
                    println!("Output written to {}", output_path);
                },
                Err(err) => {
                    error!("Failed to write resolution result: {}", err);
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            }
        }
        Some(Commands::Version) => {
            println!("git-merge-ai-resolver version {}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::Doctor(args)) => {
            info!("Running diagnostics");
            println!("Diagnostic results will be shown here");
            // TODO: Implement doctor functionality
        }
        None => {
            // When no subcommand is provided, act as a Git merge driver
            info!("Acting as Git merge driver");
            
            // Get arguments
            let args: Vec<String> = env::args().skip(1).collect();
            
            if args.len() < 4 {
                error!("Insufficient arguments for Git merge driver");
                eprintln!("Error: Insufficient arguments for Git merge driver");
                process::exit(1);
            }
            
            // Parse Git merge driver arguments
            match git_integration::parse_merge_driver_args(&args) {
                Ok(paths) => {
                    info!("Processing merge for file: {}", paths.conflict_path);
                    
                    // Process the merge
                    let exit_code = git_integration::process_merge(&paths);
                    process::exit(exit_code);
                }
                Err(err) => {
                    error!("Failed to parse Git merge driver arguments: {}", err);
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            }
        }
    }
}