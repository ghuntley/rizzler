# Architecture

## Overview

The git-merge-ai-resolver is a Git merge driver written in Rust that automatically resolves merge conflicts using AI techniques. It integrates with Git's custom merge driver system to handle merge conflicts programmatically rather than requiring manual resolution.

## System Components

1. **Git Integration Layer**
   - Implements the Git merge driver interface
   - Receives conflicting file versions from Git
   - Returns resolution status back to Git

2. **Configuration Manager**
   - Reads settings from global `.gitconfig` and repository-specific `.gitconfig`
   - Manages file extension associations with our merge driver
   - Handles configuration of AI model parameters and resolution strategies

3. **Conflict Parser**
   - Parses Git conflict markers in files
   - Extracts "ours", "theirs", and base versions of conflicting regions
   - Provides structured conflict data to the resolution engine

4. **Resolution Engine**
   - Implements various resolution strategies (rule-based, AI-based)
   - Selects appropriate strategy based on file type and configuration
   - Produces merged content that resolves conflicts

5. **File Manager**
   - Collects all files involved in the merge
   - Identifies files with conflicts
   - Writes resolved content back to the original locations

## Performance Considerations

- Resolution should complete within reasonable time (<5s for typical conflicts)
- Memory usage should be bounded and reasonable
- AI model selection should consider performance/quality tradeoffs

## Error Handling

- Graceful fallback to manual resolution if automatic resolution fails
- Clear error messages to help diagnose resolution failures
- Option to retry with different strategies