# git-merge-ai-resolver

A Git merge driver that automatically resolves merge conflicts using AI techniques.

## Overview

The git-merge-ai-resolver integrates with Git as a custom merge driver, allowing it to automatically resolve conflicts during merge operations. It supports various AI providers (OpenAI, Claude, Gemini, AWS Bedrock) and includes fallback mechanisms to ensure reliable conflict resolution.

## Features

- Automatic resolution of merge conflicts using AI
- Support for multiple AI providers (OpenAI, Claude, Gemini, AWS Bedrock)
- Fallback mechanisms between providers to ensure reliability
- Intelligent context gathering for better resolution quality
- Configurable per file type resolution strategies
- Built-in rule-based strategies for simple conflicts (e.g., whitespace-only changes)

## Installation

### From Binaries

1. Download the latest release for your platform from the [releases page](https://github.com/ghuntley/git-merge-ai-resolver/releases).
2. Make the binary executable and move it to a location in your PATH.

### From Source

```bash
cargo install --path .
```

## Setup

### Global Configuration

Run the setup command to configure git-merge-ai-resolver globally:

```bash
git-merge-ai-resolver setup --global
```

This will configure git-merge-ai-resolver as a merge driver in your global Git configuration and associate it with common file extensions.

### Repository-Specific Configuration

For a single repository:

```bash
git-merge-ai-resolver setup --local
```

### File Type Configuration

Specify which file types should use git-merge-ai-resolver:

```bash
git-merge-ai-resolver setup --extensions js,py,rs,go,java,c,cpp,h,hpp,md,txt
```

## Configuration

### Environment Variables

#### AI Provider Configuration

- `GIT_MERGE_AI_PROVIDER`: Select default AI provider ("openai", "claude", "gemini", "bedrock")
- `GIT_MERGE_AI_USE_FALLBACK`: Enable fallback between providers ("true"/"false")
- `GIT_MERGE_AI_FALLBACK_ORDER`: Comma-separated list of providers to try in order (e.g., "openai,claude,gemini,bedrock")

#### OpenAI

- `GIT_MERGE_OPENAI_API_KEY`: OpenAI API key
- `GIT_MERGE_OPENAI_BASE_URL`: Custom API endpoint URL (optional)
- `GIT_MERGE_OPENAI_MODEL`: Model to use (default: "gpt-4")

#### Claude

- `GIT_MERGE_CLAUDE_API_KEY`: Claude API key
- `GIT_MERGE_CLAUDE_MODEL`: Model to use (default: "claude-3-opus-20240229")

#### Gemini

- `GIT_MERGE_GEMINI_API_KEY`: Gemini API key
- `GIT_MERGE_GEMINI_MODEL`: Model to use (default: "gemini-pro")

#### AWS Bedrock

- `AWS_ACCESS_KEY_ID`: AWS access key ID
- `AWS_SECRET_ACCESS_KEY`: AWS secret access key
- `AWS_REGION`: AWS region
- `GIT_MERGE_BEDROCK_MODEL`: Model to use (default: "anthropic.claude-3-opus-20240229")

#### System Prompt

- `GIT_MERGE_AI_SYSTEM_PROMPT`: Custom system prompt to override the default

## Usage

Once configured, git-merge-ai-resolver will be automatically invoked by Git when a merge conflict occurs in a file with a configured extension.

### Manual Resolution

To manually resolve conflicts in a file:

```bash
git-merge-ai-resolver resolve path/to/file --output resolved.txt
```

### Doctor Command

Verify your configuration and diagnose issues:

```bash
git-merge-ai-resolver doctor
```

## License

MIT License