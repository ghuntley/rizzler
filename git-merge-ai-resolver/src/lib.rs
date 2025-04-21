// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

pub mod config;
pub mod conflict_parser;
pub mod git_integration;

// Re-export main structures for easier access
pub use config::Config;
pub use conflict_parser::{ConflictFile, ConflictRegion};
pub use git_integration::{MergeDriverPaths, parse_merge_driver_args, process_merge};