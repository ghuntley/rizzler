// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::ai_provider::AIProvider;
use crate::conflict_parser::{ConflictFile, ConflictRegion};
use crate::resolution_engine::{ResolutionError, ResolutionStrategy};
use tracing::{debug, info};

/// The `WindowingStrategy` enhances AI resolution for large files
/// by breaking them into smaller chunks that fit within token limits
pub struct WindowingStrategy {
    /// The underlying AI provider
    provider: Box<dyn AIProvider>,
    
    /// Maximum number of context lines to include around each conflict
    max_context_lines: usize,
}

impl WindowingStrategy {
    /// Create a new windowing strategy with a specific AI provider
    pub fn new(provider: Box<dyn AIProvider>, max_context_lines: usize) -> Self {
        WindowingStrategy {
            provider,
            max_context_lines,
        }
    }
    
    /// Resolve all conflicts in a file using windowing for context
    pub fn resolve_file(&self, conflict_file: &ConflictFile) -> Result<String, ResolutionError> {
        info!("Using windowing strategy to resolve {} conflicts in {}", 
              conflict_file.conflicts.len(), conflict_file.path);
        
        if conflict_file.conflicts.is_empty() {
            return Ok(conflict_file.content.clone());
        }
        
        let file_lines: Vec<&str> = conflict_file.content.lines().collect();
        let mut resolved_content = String::new();
        let mut last_end = 0;
        
        // Process each conflict with appropriate context
        for conflict in &conflict_file.conflicts {
            // Add content from last processed position to start of this conflict's context
            let context_start = if conflict.start_line > self.max_context_lines {
                conflict.start_line - self.max_context_lines
            } else {
                1 // Line numbers are 1-indexed
            };
            
            // Add non-conflict content between last processed position and start of this context
            if last_end > 0 && context_start > last_end {
                for i in last_end..(context_start-1) {
                    if i-1 < file_lines.len() { // Adjust for 0-indexing of arrays
                        resolved_content.push_str(file_lines[i-1]);
                        resolved_content.push('\n');
                    }
                }
            } else if last_end == 0 {
                // First conflict, add all content before context start
                for i in 0..(context_start-1) {
                    if i < file_lines.len() {
                        resolved_content.push_str(file_lines[i]);
                        resolved_content.push('\n');
                    }
                }
            }
            
            // Create windowed conflict file with limited context
            let windowed_file = self.create_windowed_conflict_file(
                conflict_file, 
                conflict, 
                context_start,
            );
            
            // Try to resolve this windowed conflict
            match self.provider.resolve_conflict(&windowed_file, conflict) {
                Ok(response) => {
                    // Add resolved content
                    resolved_content.push_str(&response.content);
                    if !response.content.ends_with('\n') {
                        resolved_content.push('\n');
                    }
                    debug!("Successfully resolved conflict at lines {}-{}", 
                           conflict.start_line, conflict.end_line);
                },
                Err(err) => {
                    return Err(ResolutionError::StrategyError(
                        format!("Failed to resolve conflict at lines {}-{}: {}", 
                                conflict.start_line, conflict.end_line, err)
                    ));
                }
            }
            
            // Update last processed position
            last_end = conflict.end_line;
        }
        
        // Add any remaining content after the last conflict
        if last_end > 0 && last_end-1 < file_lines.len() {
            for i in (last_end-1)..file_lines.len() {
                resolved_content.push_str(file_lines[i]);
                resolved_content.push('\n');
            }
        }
        
        // Remove trailing newline if original didn't have one
        if !conflict_file.content.ends_with('\n') && resolved_content.ends_with('\n') {
            resolved_content.pop();
        }
        
        info!("Successfully resolved all conflicts using windowing strategy");
        Ok(resolved_content)
    }
    
    /// Creates a windowed view of the conflict file with limited context
    fn create_windowed_conflict_file(
        &self,
        original_file: &ConflictFile,
        conflict: &ConflictRegion,
        context_start: usize,
    ) -> ConflictFile {
        // Calculate context end
        let context_end = conflict.end_line + self.max_context_lines;
        
        // Extract relevant lines for the windowed view
        let file_lines: Vec<&str> = original_file.content.lines().collect();
        let start_idx = context_start.saturating_sub(1); // Convert to 0-indexed
        let end_idx = context_end.min(file_lines.len());
        
        // Create windowed content
        let windowed_content = file_lines[start_idx..end_idx].join("\n");
        
        // Create the windowed conflict file
        ConflictFile {
            path: original_file.path.clone(),
            conflicts: vec![conflict.clone()],
            content: windowed_content,
        }
    }
    
    /// Estimate token count based on character count
    /// This is a rough approximation - in a real implementation, you would use
    /// a proper tokenizer based on the model being used
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough approximation: 4 characters per token on average
        (text.len() as f64 / 4.0).ceil() as usize
    }
}

impl ResolutionStrategy for WindowingStrategy {
    fn name(&self) -> &str {
        "ai-windowing"
    }
    
    fn can_handle(&self, _conflict: &ConflictRegion) -> bool {
        // Can handle any conflict as long as the provider is available
        self.provider.is_available()
    }
    
    fn resolve_conflict(&self, conflict: &ConflictRegion) -> Result<String, ResolutionError> {
        // Create a minimal conflict file with just this conflict
        let conflict_file = ConflictFile {
            path: "file.txt".to_string(), // Placeholder path
            conflicts: vec![conflict.clone()],
            content: format!(
                "<<<<<<< HEAD\n{}=======\n{}>>>>>>> branch-name",
                conflict.our_content,
                conflict.their_content
            ),
        };
        
        // For a single conflict, we don't need windowing, just pass through to provider
        match self.provider.resolve_conflict(&conflict_file, conflict) {
            Ok(response) => Ok(response.content),
            Err(err) => Err(ResolutionError::StrategyError(
                format!("Failed to resolve conflict: {}", err)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_provider::{AIProviderConfig, TokenUsage};
    use std::collections::HashMap;
    
    // Mock AI provider for testing
    struct MockAIProvider {
        max_context_size: usize,
    }
    
    impl MockAIProvider {
        fn new() -> Self {
            MockAIProvider {
                max_context_size: 1000, // 1000 tokens mock limit
            }
        }
    }
    
    impl AIProvider for MockAIProvider {
        fn name(&self) -> &str {
            "Mock Provider"
        }
        
        fn is_available(&self) -> bool {
            true
        }
        
        fn config(&self) -> &crate::ai_provider::AIProviderConfig {
            // We don't need this for the mock
            panic!("Not implemented for mock")
        }
        
        fn resolve_conflict(
        &self,
        conflict_file: &ConflictFile,
        conflict: &ConflictRegion,
        ) -> Result<AIResponse, AIProviderError> {
        // For testing purposes, make the mock always succeed
        // This is because we're mainly testing the windowing mechanism, not the provider
            
            // For testing, just return a response that includes the line number
            let line_number = conflict.start_line;
            Ok(AIResponse {
                content: format!("Resolved content at line {}\n", line_number),
                explanation: Some(format!("Resolved conflict at line {}", line_number)),
                token_usage: Some(TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    total_tokens: 150,
                }),
                model: "mock-model".to_string(),
            })
        }
        
        fn resolve_file(
        &self,
        conflict_file: &ConflictFile,
        ) -> Result<AIResponse, AIProviderError> {
        // For testing purposes, make the mock always succeed
        // This is because we're mainly testing the windowing mechanism, not the provider
            
            // For testing, return a response that includes all conflict line numbers
            let mut content = String::new();
            for conflict in &conflict_file.conflicts {
                content.push_str(&format!("Resolved content at line {}\n", conflict.start_line));
            }
            
            Ok(AIResponse {
                content,
                explanation: Some("Resolved all conflicts".to_string()),
                token_usage: Some(TokenUsage {
                    input_tokens: 200,
                    output_tokens: 100,
                    total_tokens: 300,
                }),
                model: "mock-model".to_string(),
            })
        }
    }
    
    #[test]
    fn test_windowing_strategy_single_conflict() {
        // Create a mock conflict
        let conflict = ConflictRegion {
            base_content: "Base content\n".to_string(),
            our_content: "Our content\n".to_string(),
            their_content: "Their content\n".to_string(),
            start_line: 10,
            end_line: 14,
        };
        
        // Create a conflict file with this conflict
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\n\
<<<<<<< HEAD\nOur content\n=======\nTheir content\n>>>>>>> branch-name\n\
Line 15\nLine 16\nLine 17\nLine 18\nLine 19\nLine 20\n";
        
        let conflict_file = ConflictFile {
            path: "test.txt".to_string(),
            conflicts: vec![conflict],
            content: content.to_string(),
        };
        
        // Create windowing strategy
        let windowing = WindowingStrategy::new(Box::new(MockAIProvider::new()), 5);
        
        // Test resolving file
        let result = windowing.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Check the resolved content
        let resolved = result.unwrap();
        assert!(resolved.contains("Resolved content at line 10"));
    }
    
    #[test]
    fn test_windowing_strategy_multiple_conflicts() {
        // Create a mock file with multiple conflicts
        let mut content = String::new();
        let mut conflicts = Vec::new();
        
        // Create file content with conflicts at specific lines
        for i in 1..100 {
            if i == 20 || i == 50 || i == 80 {
                // Add conflict at these positions
                content.push_str(&format!("<<<<<<< HEAD\nOur content at line {}\n=======\nTheir content at line {}\n>>>>>>> branch-name\n", i, i));
                
                // Add conflict to the list
                conflicts.push(ConflictRegion {
                    base_content: format!("Base content at line {}\n", i),
                    our_content: format!("Our content at line {}\n", i),
                    their_content: format!("Their content at line {}\n", i),
                    start_line: i,
                    end_line: i + 4,
                });
            } else {
                content.push_str(&format!("Line {}\n", i));
            }
        }
        
        // Create conflict file
        let conflict_file = ConflictFile {
            path: "test_multiple.txt".to_string(),
            conflicts,
            content,
        };
        
        // Create windowing strategy
        let windowing = WindowingStrategy::new(Box::new(MockAIProvider::new()), 10);
        
        // Test resolving file
        let result = windowing.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Check the resolved content
        let resolved = result.unwrap();
        assert!(resolved.contains("Resolved content at line 20"));
        assert!(resolved.contains("Resolved content at line 50"));
        assert!(resolved.contains("Resolved content at line 80"));
    }
    
    #[test]
    fn test_windowing_strategy_large_files() {
        // Create a mock large file with conflicts
        let mut content = String::new();
        let mut conflicts = Vec::new();
        
        // Create a large file (5K lines)
        for i in 1..5_000 {
            if i == 1000 || i == 2500 || i == 4000 {
                // Add conflict at these positions
                content.push_str(&format!("<<<<<<< HEAD\nOur content at line {}\n=======\nTheir content at line {}\n>>>>>>> branch-name\n", i, i));
                
                // Add conflict to the list
                conflicts.push(ConflictRegion {
                    base_content: format!("Base content at line {}\n", i),
                    our_content: format!("Our content at line {}\n", i),
                    their_content: format!("Their content at line {}\n", i),
                    start_line: i,
                    end_line: i + 4,
                });
            } else {
                content.push_str(&format!("Line {}\n", i));
            }
        }
        
        // Create conflict file
        let conflict_file = ConflictFile {
            path: "large_file.txt".to_string(),
            conflicts,
            content,
        };
        
        // Create mock AI provider with small context limit to force windowing
        let mock_provider = MockAIProvider::new(); // Only 1000 chars context
        
        // Create windowing strategy with small window to force chunking
        let windowing = WindowingStrategy::new(Box::new(mock_provider), 100);
        
        // Test resolving file
        let result = windowing.resolve_file(&conflict_file);
        assert!(result.is_ok());
        
        // Check the resolved content
        let resolved = result.unwrap();
        assert!(resolved.contains("Resolved content at line 1000"));
        assert!(resolved.contains("Resolved content at line 2500"));
        assert!(resolved.contains("Resolved content at line 4000"));
    }
}