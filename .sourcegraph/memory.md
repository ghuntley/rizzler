# Rizzler Project Memory

## Commands

### Build, Test and Run Commands

- Build the project: `cargo build`
- Run all tests: `cargo test`
- Run a specific test: `cargo test <test_name>`
- Check for errors: `cargo check`
- Format code: `rustfmt`

### Custom Scripts

- Resolve merge conflicts: `scripts/resolve_merge_conflicts.sh <file_path>`
- Check for merge conflict markers: `scripts/check_merge_conflicts.sh <file_path>`
- Test conflict resolution with backup/restore: `scripts/test_resolve_conflicts.sh`

## Important Environment Variables

- `RIZZLER_CLAUDE_API_KEY`: API key for Claude integration
- `RIZZLER_OPENAI_API_KEY`: API key for OpenAI integration
- `RIZZLER_PROVIDER`: Set to "claude" or "openai" to select AI provider
- `RIZZLER_RUN_INTEGRATION_TESTS`: Set to "true" to enable integration tests

## Project Structure

- `src/`: Main source code
  - `src/providers/`: AI provider implementations
  - `src/bin/`: Binary executables
- `tests/`: Test files
- `examples/`: Example files and test data
- `scripts/`: Utility scripts

## Notes

- Always backup and restore files when doing merge conflict resolution
- Source the `~/.profile` file to get API keys before running tests