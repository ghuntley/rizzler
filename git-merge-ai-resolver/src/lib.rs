// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

pub mod ai_provider;
pub mod ai_resolution;
pub mod ai_resolution_windowing;
pub mod config;
pub mod conflict_parser;
pub mod fallback;
pub mod git_integration;
pub mod providers;
pub mod resolution_engine;
pub mod windowing;

// Re-export main structures for easier access
pub use config::Config;
pub use conflict_parser::{ConflictFile, ConflictRegion, parse_conflict_file, parse_conflict_file_with_base, parse_conflict_file_with_context_matching};
pub use git_integration::{MergeDriverPaths, parse_merge_driver_args, process_merge};
pub use resolution_engine::{ResolutionEngine, ResolutionStrategy, ResolutionResult};
pub use ai_provider::{AIProvider, AIProviderError, AIResponse};
pub use providers::{OpenAIProvider, ClaudeProvider, GeminiProvider, BedrockProvider};
pub use ai_resolution::{AIResolutionStrategy, AIFileResolutionStrategy};
pub use ai_resolution_windowing::{AIResolutionWithWindowingStrategy, AIFileResolutionWithWindowingStrategy};
pub use fallback::FallbackResolutionStrategy;
pub use windowing::WindowingStrategy;