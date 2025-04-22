// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

pub mod ai_provider;
pub mod ai_resolution;
pub mod ai_resolution_windowing;
pub mod cache;
pub mod caching_provider;
pub mod config;
pub mod conflict_parser;
pub mod diagnostics;
pub mod fallback;
pub mod git_integration;
pub mod git_setup;
pub mod providers;
pub mod prompt_engineering;
pub mod resolution_engine;
pub mod retry;
pub mod windowing;

// Test modules
#[cfg(test)]
mod cache_disk_tests;

// Re-export main structures for easier access
pub use ai_provider::{AIProvider, AIProviderError, AIResponse};
pub use ai_resolution::{AIResolutionStrategy, AIFileResolutionStrategy};
pub use ai_resolution_windowing::{AIResolutionWithWindowingStrategy, AIFileResolutionWithWindowingStrategy};
pub use cache::AIResolutionCache;
pub use caching_provider::CachingAIProvider;
pub use config::Config;
pub use conflict_parser::{ConflictFile, ConflictRegion, parse_conflict_file, parse_conflict_file_with_base, parse_conflict_file_with_context_matching};
pub use diagnostics::{DiagnosticResult, DiagnosticStatus, run_diagnostics, format_diagnostic_results, write_diagnostic_results};
pub use fallback::FallbackResolutionStrategy;
pub use git_integration::{MergeDriverPaths, parse_merge_driver_args, process_merge};
pub use git_setup::{setup_git_integration, SetupError};
pub use prompt_engineering::{PromptGenerator, PromptTemplate};
pub use providers::{OpenAIProvider, ClaudeProvider, GeminiProvider, BedrockProvider};
pub use resolution_engine::{ResolutionEngine, ResolutionStrategy, ResolutionResult};
pub use retry::{RetryableProvider, RetryConfig};
pub use windowing::WindowingStrategy;