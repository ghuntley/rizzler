// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

pub mod ai_provider;
pub mod ai_resolution;
pub mod config;
pub mod conflict_parser;
pub mod git_integration;
pub mod resolution_engine;

// Re-export main structures for easier access
pub use config::Config;
pub use conflict_parser::{ConflictFile, ConflictRegion};
pub use git_integration::{MergeDriverPaths, parse_merge_driver_args, process_merge};
pub use resolution_engine::{ResolutionEngine, ResolutionStrategy, ResolutionResult};
pub use ai_provider::{AIProvider, AIProviderError, AIResponse, OpenAIProvider};
pub use ai_resolution::{AIResolutionStrategy, AIFileResolutionStrategy};