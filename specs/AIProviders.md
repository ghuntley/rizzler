# AI Provider Integration

## Overview

The rizzler will support multiple AI providers to give users flexibility in choosing which models to use for conflict resolution.

## Supported Providers

### OpenAI
- Environment variables:
  - `RIZZLER_OPENAI_API_KEY`: API key for authentication
  - `RIZZLER_OPENAI_BASE_URL`: Custom API endpoint URL (optional)
  - `RIZZLER_OPENAI_ORG_ID`: Organization ID (optional)
- Models supported: GPT-3.5-turbo, GPT-4, GPT-4-turbo
- Custom endpoint support:
  - Azure OpenAI Service
  - Self-hosted compatible endpoints (e.g., llama.cpp server)
  - Enterprise endpoints

### Anthropic (Claude)
- Environment variable: `RIZZLER_CLAUDE_API_KEY`
- Models supported: Claude 3 Opus, Sonnet, Haiku
- Optional configuration parameters:
  - Base URL (for enterprise endpoints)

### Google (Gemini)
- Environment variable: `RIZZLER_GEMINI_API_KEY`
- Models supported: Gemini Pro, Gemini Ultra
- Optional configuration parameters:
  - Project ID
  - Location

### AWS Bedrock
- Authentication via AWS credentials chain:
  - Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
  - AWS configuration files
  - IAM roles for EC2/ECS
- Models supported:
  - Anthropic Claude models on Bedrock
  - Amazon Titan models
  - Other models available through Bedrock
- Required configuration parameters:
  - AWS Region

## AI Interaction Flow

1. The AI Resolution Service:
   - Connects to configured AI model
   - Uploads all files involved in the merge to the LLM endpoint
   - Specifically identifies files with conflicts to the model
   - Prompts the LLM to resolve the conflicts in the identified files
   - Processes AI responses to generate resolved content
   - Writes the resolved content back to original file locations

## System Prompt Configuration

- Environment variable: `RIZZLER_SYSTEM_PROMPT` - Override the default system prompt
- Default system prompt will instruct the AI to:
  - Analyze all files involved in the merge
  - Pay special attention to identified files with conflicts
  - Resolve conflicts sensibly based on the context of changes
  - Preserve semantics and functionality
  - Explain reasoning for conflict resolutions

## Caching Configuration

The rizzler implements a disk-based caching system to improve performance and reduce API costs by avoiding redundant AI calls.

### Configuration File Options

In the `rizzler.toml` configuration file, you can specify caching options under the `[cache]` section:

```toml
[cache]
# Enable or disable the cache system
enabled = true

# Directory to store cache files
directory = "~/.cache/rizzler" 

# Cache time-to-live in hours
ttl_hours = 24

# Maximum number of entries per cache type (conflicts and files)
max_entries = 1000

# Enable automatic cleanup of expired entries during cache operations
auto_cleanup = true

# Flush cache to disk immediately after writing
immediate_flush = false
```

### Environment Variables

These environment variables override the settings in the configuration file:

- `RIZZLER_USE_CACHE`: Enable/disable caching (true/false, default: true)
- `RIZZLER_CACHE_DIR`: Directory to store cache files (default: system temp dir)
- `RIZZLER_CACHE_TTL_HOURS`: Time-to-live for cache entries in hours (default: 24)
- `RIZZLER_CACHE_MAX_ENTRIES`: Maximum number of entries per cache type
- `RIZZLER_CACHE_AUTO_CLEANUP`: Enable/disable automatic cleanup (true/false, default: true)
- `RIZZLER_CACHE_IMMEDIATE_FLUSH`: Enable/disable immediate disk flush (true/false, default: false)

### Cache Features

- Persistent disk-based storage of AI responses
- Automatic cleanup of expired entries
- Configurable maximum number of entries
- Caching of both individual conflict resolutions and whole-file resolutions
- Hash-based cache keys for efficient lookup
- Error-resistant design with fallback to AI on cache failures
- Optional immediate disk flushing for enhanced reliability
- Support for cache persistence across program executions