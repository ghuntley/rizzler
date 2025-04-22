# Core Architecture

## Overview

The rizzler is a Git merge driver written in Rust that automatically resolves merge conflicts using AI techniques. It integrates with Git's custom merge driver system to handle merge conflicts programmatically rather than requiring manual resolution.

## Components

1. **Git Integration Layer**
   - Implements the Git merge driver interface
   - Receives conflicting file versions from Git
   - Returns resolved content back to Git

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

5. **AI Resolution Service**
   - Connects to local or remote AI models
   - Uploads all files involved in the merge to the LLM endpoint
   - Specifically identifies files with conflicts to the model
   - Prompts the LLM to resolve the conflicts in the identified files
   - Formats conflict data for AI processing
   - Interprets AI responses to generate resolved content
   - Writes the resolved content back to the original conflict file locations
   - Supports customizable system prompts via environment variables

6. **Logging and Telemetry**
   - Records resolution decisions and success/failure metrics
   - Provides debugging information
   - Optional anonymized usage data for improvement (opt-in)

## Data Flow

1. Git invokes our merge driver with paths to the conflicting versions of a file
2. Configuration is loaded based on file type and user settings
3. All files involved in the merge are collected
4. Conflict regions are parsed from conflicted files
5. Resolution engine selects and applies appropriate strategy
6. If using AI resolution:
   - All merge files are uploaded to the LLM endpoint
   - Files with conflicts are specifically identified to the model
   - LLM is prompted to resolve the conflicts in these files
   - Custom system prompt is applied (if configured)
   - Conflict data is processed by the AI service
7. Resolved content is written back to the filesystem at the original conflict file locations
8. Successful resolution status is returned to Git
9. Resolution metrics are logged

## Performance Considerations

- Resolution should complete within reasonable time (<5s for typical conflicts)
- Memory usage should be bounded and reasonable
- AI model selection should consider performance/quality tradeoffs

## Error Handling

- Graceful fallback to manual resolution if automatic resolution fails
- Clear error messages to help diagnose resolution failures
- Option to retry with different strategies