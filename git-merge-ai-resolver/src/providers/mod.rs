// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

pub mod claude;
pub mod openai;

// Re-export providers for easier access
pub use claude::ClaudeProvider;
pub use openai::OpenAIProvider;