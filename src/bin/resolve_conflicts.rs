// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use rizzler::ai_provider::AIProvider;
use rizzler::conflict_parser::parse_conflict_file;
use rizzler::providers::claude::ClaudeProvider;
use rizzler::providers::openai::OpenAIProvider;
use rizzler::providers::bedrock::BedrockProvider;
use rizzler::providers::gemini::GeminiProvider;

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::exit;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Check for required command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <conflict_file_path>", args[0]);
        exit(1);
    }
    
    // Get the file path from arguments
    let file_path = &args[1];
    
    // Backup functionality removed
    
    // Read the file content
    let mut file_content = String::new();
    File::open(file_path)
        .and_then(|mut file| file.read_to_string(&mut file_content))
        .expect("Failed to read conflict file");
    
    println!("Parsing conflicts in {}", file_path);
    
    // Parse the conflict file
    let conflict_file = match parse_conflict_file(file_path) {
        Ok(cf) => cf,
        Err(e) => {
            eprintln!("Error parsing conflict file: {:?}", e);
            // Restore from backup not needed as we haven't modified anything yet
            exit(1);
        }
    };
    
    println!("Found {} conflicts", conflict_file.conflicts.len());
    
    // Determine which AI provider to use
    let provider_name = env::var("RIZZLER_PROVIDER").unwrap_or_else(|_| "claude".to_string());
    
    let resolution_result = match provider_name.as_str() {
        "claude" => {
            println!("Using Claude provider");
            match ClaudeProvider::new() {
                Ok(provider) => provider.resolve_file(&conflict_file),
                Err(e) => {
                    eprintln!("Error creating Claude provider: {:?}", e);
                    // Restore functionality removed
                    exit(1);
                }
            }
        },
        "openai" => {
            println!("Using OpenAI provider");
            match OpenAIProvider::new() {
                Ok(provider) => provider.resolve_file(&conflict_file),
                Err(e) => {
                    eprintln!("Error creating OpenAI provider: {:?}", e);
                    // Restore functionality removed
                    exit(1);
                }
            }
        },
        "bedrock" | "aws" => {
            println!("Using AWS Bedrock provider");
            match BedrockProvider::new() {
                Ok(provider) => provider.resolve_file(&conflict_file),
                Err(e) => {
                    eprintln!("Error creating AWS Bedrock provider: {:?}", e);
                    // Restore functionality removed
                    exit(1);
                }
            }
        },
        "gemini" | "google" => {
            println!("Using Google Gemini provider");
            match GeminiProvider::new() {
                Ok(provider) => provider.resolve_file(&conflict_file),
                Err(e) => {
                    eprintln!("Error creating Gemini provider: {:?}", e);
                    // Restore functionality removed
                    exit(1);
                }
            }
        },
        _ => {
            eprintln!("Unsupported provider: {}", provider_name);
            // Restore functionality removed
            exit(1);
        }
    };
    
    // Process the resolution result
    match resolution_result {
        Ok(response) => {
            println!("Successfully resolved conflicts");
            
            // Check that the resolved content doesn't contain conflict markers
            if response.content.contains("<<<<<<< HEAD") || 
               response.content.contains("=======") || 
               response.content.contains(">>>>>>>")
            {
                eprintln!("Error: Resolved content still contains conflict markers");
                // Restore functionality removed
                exit(1);
            }
            
            // Write the resolved content back to the file
            match File::create(file_path) {
                Ok(mut file) => {
                    match file.write_all(response.content.as_bytes()) {
                        Ok(_) => {
                            println!("Successfully wrote resolved content to {}", file_path);
                            println!("Token usage: {:?}", response.token_usage);
                            
                            // Backup/restore functionality removed
                            Ok(())
                        },
                        Err(e) => {
                            eprintln!("Error writing resolved content to file: {}", e);
                            // Restore functionality removed
                            exit(1);
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Error creating file for writing: {}", e);
                    // Restore functionality removed
                    exit(1);
                }
            }
        },
        Err(e) => {
            eprintln!("Error resolving conflicts: {:?}", e);
            // Restore functionality removed
            exit(1);
        }
    }
}

// Restore functionality has been removed