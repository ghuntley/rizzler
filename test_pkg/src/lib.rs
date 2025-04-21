// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

// Re-export functions and types from main.rs
pub use crate::main::{ConflictFile, ConflictParseError, ConflictRegion, parse_conflict_file, parse_conflict_file_with_base, parse_conflict_file_with_context_matching};

// Include the main module
mod main;