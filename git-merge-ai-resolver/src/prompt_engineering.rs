// Copyright (c) 2025 Geoffrey Huntley
// SPDX-License-Identifier: MIT

use crate::conflict_parser::{ConflictFile, ConflictRegion};
use std::env;
use tracing::{debug, info};

/// Template variants for prompt generation
pub enum PromptTemplate {
    /// Default basic template with minimal formatting
    Default,
    /// Enhanced template with additional guidance for code analysis
    Enhanced,
    /// Context-aware template that includes BASE version and surrounding context
    ContextAware,
}

/// Generator for creating optimized prompts for AI model consumption
pub struct PromptGenerator {
    template: PromptTemplate,
}

impl PromptGenerator {
    /// Create a new PromptGenerator with the specified template
    pub fn new(template: PromptTemplate) -> Self {
        PromptGenerator { template }
    }
    
    /// Generate a system prompt for AI model instruction
    pub fn generate_system_prompt(&self) -> String {
        // Check if a custom system prompt is provided in the environment
        if let Ok(custom_prompt) = env::var("GIT_MERGE_AI_SYSTEM_PROMPT") {
            return custom_prompt;
        }
        
        // Otherwise, use the template-specific system prompt
        match self.template {
            PromptTemplate::Default => self.default_system_prompt(),
            PromptTemplate::Enhanced => self.enhanced_system_prompt(),
            PromptTemplate::ContextAware => self.context_aware_system_prompt(),
        }
    }
    
    /// Generate a prompt for resolving a specific conflict
    pub fn generate_conflict_prompt(&self, conflict_file: &ConflictFile, conflict: &ConflictRegion) -> String {
        match self.template {
            PromptTemplate::Default => self.default_conflict_prompt(conflict_file, conflict),
            PromptTemplate::Enhanced => self.enhanced_conflict_prompt(conflict_file, conflict),
            PromptTemplate::ContextAware => self.context_aware_conflict_prompt(conflict_file, conflict),
        }
    }
    
    /// Generate a prompt for resolving an entire file with multiple conflicts
    pub fn generate_file_prompt(&self, conflict_file: &ConflictFile) -> String {
        match self.template {
            PromptTemplate::Default => self.default_file_prompt(conflict_file),
            PromptTemplate::Enhanced => self.enhanced_file_prompt(conflict_file),
            PromptTemplate::ContextAware => self.context_aware_file_prompt(conflict_file),
        }
    }
    
    /// Default system prompt focused on basic conflict resolution
    fn default_system_prompt(&self) -> String {
        "You are an expert software developer helping to resolve Git merge conflicts. \
        Analyze the provided code conflicts and resolve them in a way that preserves \
        the intent of both changes whenever possible. When resolving conflicts, consider \
        the context of the entire file and follow the existing code style. Provide a \
        clean resolution without conflict markers.".to_string()
    }
    
    /// Enhanced system prompt with additional guidance for semantic understanding
    fn enhanced_system_prompt(&self) -> String {
        "You are an expert software developer specializing in resolving Git merge conflicts. \
        Your task is to analyze code conflicts with deep semantic understanding and resolve them \
        to preserve the functionality and intent of both changes. When resolving conflicts:
        
        1. Analyze the semantic meaning of both versions, not just textual differences
        2. Preserve functional changes from both sides when possible
        3. Prioritize correctness and maintaining program logic over simple text merging
        4. Follow the coding style of the existing codebase
        5. Consider edge cases and potential side effects of your resolution
        6. If both sides make incompatible changes, make a reasonable choice and explain why
        
        Provide only the cleanly resolved code without conflict markers or explanations unless requested.".to_string()
    }
    
    /// Context-aware system prompt that emphasizes the importance of base version and context
    fn context_aware_system_prompt(&self) -> String {
        "You are an expert software developer specializing in resolving Git merge conflicts. \
        Your task is to analyze code conflicts with deep semantic understanding and resolve them \
        to preserve the functionality and intent of both changes. When resolving conflicts:
        
        1. Compare both versions to the original BASE version to understand what each change is trying to accomplish
        2. Analyze the surrounding context of the file to understand how the conflicting code interacts with the rest
        3. Preserve functional changes from both sides when possible
        4. Prioritize correctness and maintaining program logic over simple text merging
        5. Follow the coding style of the existing codebase
        6. Look for complementary changes that should be combined rather than chosen between
        7. If both sides make incompatible changes, make a reasonable choice based on the overall context
        
        Provide only the cleanly resolved code without conflict markers or explanations unless requested.".to_string()
    }
    
    /// Default conflict prompt with basic conflict information
    fn default_conflict_prompt(&self, conflict_file: &ConflictFile, conflict: &ConflictRegion) -> String {
        format!(
            "I need help resolving a Git merge conflict in the file: {}\n\n\
            The file contains a conflict between line {} and {}:\n\n\
            OUR VERSION (current branch):\n```\n{}```\n\n\
            THEIR VERSION (incoming branch):\n```\n{}```\n\n\
            Please resolve this conflict and provide only the final resolved content that should replace \
            the conflict. Preserve the intent of both changes if possible or choose the most appropriate \
            version if they are in direct conflict. Do not include conflict markers in your response.",
            conflict_file.path,
            conflict.start_line,
            conflict.end_line,
            conflict.our_content,
            conflict.their_content
        )
    }
    
    /// Enhanced conflict prompt with additional guidance for code analysis
    fn enhanced_conflict_prompt(&self, conflict_file: &ConflictFile, conflict: &ConflictRegion) -> String {
        format!(
            "I need help resolving a Git merge conflict in the file: {}\n\n\
            CONFLICT DETAILS:\n\
            - Location: Lines {} to {}\n\
            - File type: {}\n\n\
            OUR VERSION (current branch):\n```\n{}```\n\n\
            THEIR VERSION (incoming branch):\n```\n{}```\n\n\
            CONFLICT ANALYSIS:\n\
            Please analyze the semantic differences between these versions. Look for:\n\
            1. Functional changes (what each version is trying to accomplish)\n\
            2. Complementary changes that could be combined\n\
            3. Incompatible changes that require choosing one approach\n\n\
            RESOLUTION INSTRUCTIONS:\n\
            Provide ONLY the final resolved code that should replace the conflict region.\n\
            - Do not include conflict markers, explanations, or reasoning\n\
            - Follow the project's coding style\n\
            - Preserve intent from both versions when possible\n\
            - Choose the most appropriate version if changes are incompatible",
            conflict_file.path,
            conflict.start_line,
            conflict.end_line,
            Self::determine_file_type(&conflict_file.path),
            conflict.our_content,
            conflict.their_content
        )
    }
    
    /// Context-aware conflict prompt that includes BASE version and surrounding context
    fn context_aware_conflict_prompt(&self, conflict_file: &ConflictFile, conflict: &ConflictRegion) -> String {
        // Extract the surrounding context from the file content if available
        let surrounding_context = Self::extract_surrounding_context(conflict_file, conflict);
        
        let base_content_section = if !conflict.base_content.is_empty() {
            format!("BASE VERSION (common ancestor):\n```\n{}```\n\n", conflict.base_content)
        } else {
            String::new()
        };
        
        let context_section = if !surrounding_context.is_empty() {
            format!("SURROUNDING CONTEXT:\n```\n{}```\n\n", surrounding_context)
        } else {
            String::new()
        };
        
        format!(
            "I need help resolving a Git merge conflict in the file: {}\n\n\
            CONFLICT DETAILS:\n\
            - Location: Lines {} to {}\n\
            - File type: {}\n\n\
            {}{}OUR VERSION (current branch):\n```\n{}```\n\n\
            THEIR VERSION (incoming branch):\n```\n{}```\n\n\
            RESOLUTION INSTRUCTIONS:\n\
            Analyze all versions and contexts provided above to understand the semantic differences.\n\
            Provide ONLY the final resolved code that should replace the conflict region.\n\
            - Do not include conflict markers, explanations, or reasoning\n\
            - Preserve functionality from both versions when possible\n\
            - Follow the project's coding style\n\
            - Ensure the resolution integrates well with the surrounding context",
            conflict_file.path,
            conflict.start_line,
            conflict.end_line,
            Self::determine_file_type(&conflict_file.path),
            base_content_section,
            context_section,
            conflict.our_content,
            conflict.their_content
        )
    }
    
    /// Default file prompt for resolving multiple conflicts
    fn default_file_prompt(&self, conflict_file: &ConflictFile) -> String {
        let mut conflicts_text = String::new();
        
        for (i, conflict) in conflict_file.conflicts.iter().enumerate() {
            conflicts_text.push_str(&format!(
                "CONFLICT {}:\nBetween lines {} and {}\n\
                OUR VERSION:\n```\n{}```\n\
                THEIR VERSION:\n```\n{}```\n\n",
                i + 1,
                conflict.start_line,
                conflict.end_line,
                conflict.our_content,
                conflict.their_content
            ));
        }
        
        format!(
            "I need help resolving Git merge conflicts in the file: {}\n\n\
            The file has {} conflict(s):\n\n{}\n\
            Please provide the entire resolved file content with all conflicts resolved. \
            Preserve the intent of both changes whenever possible. \
            Do not include conflict markers in your response.",
            conflict_file.path,
            conflict_file.conflicts.len(),
            conflicts_text
        )
    }
    
    /// Enhanced file prompt with additional guidance for resolving multiple conflicts
    fn enhanced_file_prompt(&self, conflict_file: &ConflictFile) -> String {
        let file_type = Self::determine_file_type(&conflict_file.path);
        let mut conflicts_text = String::new();
        
        for (i, conflict) in conflict_file.conflicts.iter().enumerate() {
            conflicts_text.push_str(&format!(
                "CONFLICT {}:\n\
                - Location: Lines {} to {}\n\
                OUR VERSION:\n```\n{}```\n\
                THEIR VERSION:\n```\n{}```\n\n",
                i + 1,
                conflict.start_line,
                conflict.end_line,
                conflict.our_content,
                conflict.their_content
            ));
        }
        
        format!(
            "I need help resolving Git merge conflicts in the file: {}\n\n\
            FILE DETAILS:\n\
            - File type: {}\n\
            - Number of conflicts: {}\n\n\
            CONFLICTS:\n\n{}\n\
            RESOLUTION INSTRUCTIONS:\n\
            Analyze each conflict and provide the entire resolved file content.\n\n\
            When resolving the conflicts:\n\
            1. Consider the semantic meaning of each change, not just textual differences\n\
            2. Preserve functional changes from both sides when possible\n\
            3. Ensure the file remains syntactically valid and logically consistent\n\
            4. Follow the coding style of the existing codebase\n\
            5. Make reasonable choices when changes are incompatible\n\n\
            Provide ONLY the final complete file content with all conflicts resolved.\n\
            Do not include conflict markers, explanations, or reasoning in your response.",
            conflict_file.path,
            file_type,
            conflict_file.conflicts.len(),
            conflicts_text
        )
    }
    
    /// Context-aware file prompt that includes BASE versions and focuses on integrated resolution
    fn context_aware_file_prompt(&self, conflict_file: &ConflictFile) -> String {
        let file_type = Self::determine_file_type(&conflict_file.path);
        let mut conflicts_text = String::new();
        
        for (i, conflict) in conflict_file.conflicts.iter().enumerate() {
            let base_content_section = if !conflict.base_content.is_empty() {
                format!("BASE VERSION:\n```\n{}```\n", conflict.base_content)
            } else {
                String::new()
            };
            
            conflicts_text.push_str(&format!(
                "CONFLICT {}:\n\
                - Location: Lines {} to {}\n\
                {}OUR VERSION:\n```\n{}```\n\
                THEIR VERSION:\n```\n{}```\n\n",
                i + 1,
                conflict.start_line,
                conflict.end_line,
                base_content_section,
                conflict.our_content,
                conflict.their_content
            ));
        }
        
        format!(
            "I need help resolving Git merge conflicts in the file: {}\n\n\
            FILE DETAILS:\n\
            - File type: {}\n\
            - Number of conflicts: {}\n\n\
            CONFLICTS:\n\n{}\n\
            RESOLUTION APPROACH:\n\
            For each conflict, compare our version and their version with the base version (if provided)\n\
            to understand what each change is trying to accomplish. Consider how the conflicts\n\
            relate to each other and ensure consistent integration.\n\n\
            RESOLUTION INSTRUCTIONS:\n\
            Provide the complete resolved file content with all conflicts resolved consistently.\n\
            - Preserve semantics and functionality from both versions when possible\n\
            - Ensure all conflicts are resolved in a coherent way\n\
            - Maintain the overall integrity and consistency of the file\n\
            - Follow the project's coding style\n\n\
            Provide ONLY the final complete file content with all conflicts resolved.\n\
            Do not include conflict markers, explanations, or reasoning in your response.",
            conflict_file.path,
            file_type,
            conflict_file.conflicts.len(),
            conflicts_text
        )
    }
    
    /// Helper function to determine the file type based on extension
    fn determine_file_type(path: &str) -> String {
        if let Some(extension) = path.split('.').last() {
            match extension.to_lowercase().as_str() {
                "rs" => "Rust".to_string(),
                "js" => "JavaScript".to_string(),
                "ts" => "TypeScript".to_string(),
                "py" => "Python".to_string(),
                "go" => "Go".to_string(),
                "java" => "Java".to_string(),
                "c" => "C".to_string(),
                "cpp" | "cc" | "cxx" => "C++".to_string(),
                "h" | "hpp" => "C/C++ Header".to_string(),
                "cs" => "C#".to_string(),
                "rb" => "Ruby".to_string(),
                "php" => "PHP".to_string(),
                "html" => "HTML".to_string(),
                "css" => "CSS".to_string(),
                "md" => "Markdown".to_string(),
                "json" => "JSON".to_string(),
                "yml" | "yaml" => "YAML".to_string(),
                "xml" => "XML".to_string(),
                "sh" | "bash" => "Shell Script".to_string(),
                "swift" => "Swift".to_string(),
                "kt" | "kts" => "Kotlin".to_string(),
                "scala" => "Scala".to_string(),
                "dart" => "Dart".to_string(),
                "r" => "R".to_string(),
                "jsx" => "React JSX".to_string(),
                "tsx" => "React TSX".to_string(),
                "vue" => "Vue.js".to_string(),
                "svelte" => "Svelte".to_string(),
                "sql" => "SQL".to_string(),
                "proto" => "Protocol Buffers".to_string(),
                "graphql" | "gql" => "GraphQL".to_string(),
                "toml" => "TOML".to_string(),
                "ini" => "INI".to_string(),
                "Dockerfile" => "Dockerfile".to_string(),
                _ => format!("File with .{} extension", extension),
            }
        } else {
            "Unknown file type".to_string()
        }
    }
    
    /// Extract surrounding context from the file content
    fn extract_surrounding_context(conflict_file: &ConflictFile, conflict: &ConflictRegion) -> String {
        // This is a simplified implementation
        // In a real implementation, we would extract non-conflicting lines around the conflict
        // For now, we'll return an empty string
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    // Helper function to create a test conflict region
    fn create_test_conflict(our_content: &str, their_content: &str) -> ConflictRegion {
        ConflictRegion {
            base_content: String::new(),
            our_content: our_content.to_string(),
            their_content: their_content.to_string(),
            start_line: 1,
            end_line: 5,
        }
    }
    
    // Helper function to create a test conflict file
    fn create_test_conflict_file(conflicts: Vec<ConflictRegion>) -> ConflictFile {
        ConflictFile {
            path: "test.txt".to_string(),
            conflicts,
            content: "<<<<<<< HEAD\nTest content\n=======\nTheir content\n>>>>>>> branch-name\n".to_string(),
        }
    }
    
    #[test]
    fn test_system_prompt_generation() {
        // Test default template
        let generator = PromptGenerator::new(PromptTemplate::Default);
        let default_prompt = generator.generate_system_prompt();
        assert!(default_prompt.contains("expert software developer"));
        
        // Test enhanced template
        let generator = PromptGenerator::new(PromptTemplate::Enhanced);
        let enhanced_prompt = generator.generate_system_prompt();
        assert!(enhanced_prompt.contains("semantic meaning"));
        
        // Test context-aware template
        let generator = PromptGenerator::new(PromptTemplate::ContextAware);
        let context_prompt = generator.generate_system_prompt();
        assert!(context_prompt.contains("BASE version"));
    }
    
    #[test]
    fn test_custom_system_prompt_from_env() {
        // Set custom prompt in environment
        env::set_var("GIT_MERGE_AI_SYSTEM_PROMPT", "Custom prompt for testing");
        
        // Create generator and check that it uses the custom prompt
        let generator = PromptGenerator::new(PromptTemplate::Default);
        let prompt = generator.generate_system_prompt();
        assert_eq!(prompt, "Custom prompt for testing");
        
        // Clean up environment
        env::remove_var("GIT_MERGE_AI_SYSTEM_PROMPT");
    }
    
    #[test]
    fn test_conflict_prompt_generation() {
        // Create a test conflict
        let conflict = create_test_conflict("function add() {}", "function add(a, b) {}");
        let conflict_file = create_test_conflict_file(vec![conflict.clone()]);
        
        // Test default template
        let generator = PromptGenerator::new(PromptTemplate::Default);
        let prompt = generator.generate_conflict_prompt(&conflict_file, &conflict);
        assert!(prompt.contains("OUR VERSION"));
        assert!(prompt.contains("THEIR VERSION"));
        
        // Test enhanced template
        let generator = PromptGenerator::new(PromptTemplate::Enhanced);
        let prompt = generator.generate_conflict_prompt(&conflict_file, &conflict);
        assert!(prompt.contains("CONFLICT ANALYSIS"));
        assert!(prompt.contains("semantic differences"));
        
        // Test context-aware template
        let generator = PromptGenerator::new(PromptTemplate::ContextAware);
        let prompt = generator.generate_conflict_prompt(&conflict_file, &conflict);
        assert!(prompt.contains("RESOLUTION INSTRUCTIONS"));
    }
    
    #[test]
    fn test_file_type_detection() {
        assert_eq!(PromptGenerator::determine_file_type("test.rs"), "Rust");
        assert_eq!(PromptGenerator::determine_file_type("app.js"), "JavaScript");
        assert_eq!(PromptGenerator::determine_file_type("config.ts"), "TypeScript");
        assert_eq!(PromptGenerator::determine_file_type("unknown.xyz"), "File with .xyz extension");
    }
    
    #[test]
    fn test_file_prompt_generation() {
        // Create multiple test conflicts
        let conflict1 = create_test_conflict("function add() {}", "function add(a, b) {}");
        let conflict2 = create_test_conflict("const x = 1;", "const x = 2;");
        let conflict_file = create_test_conflict_file(vec![conflict1, conflict2]);
        
        // Test default template
        let generator = PromptGenerator::new(PromptTemplate::Default);
        let prompt = generator.generate_file_prompt(&conflict_file);
        assert!(prompt.contains("CONFLICT 1"));
        assert!(prompt.contains("CONFLICT 2"));
        assert!(prompt.contains("has 2 conflict"));
        
        // Test enhanced template
        let generator = PromptGenerator::new(PromptTemplate::Enhanced);
        let prompt = generator.generate_file_prompt(&conflict_file);
        assert!(prompt.contains("FILE DETAILS"));
        assert!(prompt.contains("Number of conflicts: 2"));
    }
}