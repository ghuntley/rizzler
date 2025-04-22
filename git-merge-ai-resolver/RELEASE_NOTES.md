# git-merge-ai-resolver 0.1.0

## Initial Release

We're excited to announce the first release of git-merge-ai-resolver, a Git merge driver that automatically resolves merge conflicts using AI techniques.

### Key Features

- Automatic resolution of merge conflicts using AI
- Support for multiple AI providers (OpenAI, Claude, Gemini, AWS Bedrock)
- Fallback mechanisms between providers to ensure reliability
- Intelligent context gathering for better resolution quality
- Configurable per file type resolution strategies
- Built-in rule-based strategies for simple conflicts (e.g., whitespace-only changes)

### Getting Started

See the [README.md](./README.md) for installation and usage instructions.

### Known Issues

- Large files exceeding model token limits may not resolve correctly in some cases
- Some complex nested conflicts might require manual intervention
- Performance may vary depending on the selected AI provider and connection speed

### Coming Soon

- Additional rule-based resolution strategies
- Improved handling of large files
- More comprehensive logging and reporting
- Integration with additional AI providers