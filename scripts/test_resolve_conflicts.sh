#!/bin/bash

# Save current working directory
ORIGINAL_DIR=$(pwd)

# Source profile to get API keys
source ~/.profile

# Back up the original file
FILE_PATH="examples/merge_conflicts_example.sh"
BACKUP_PATH="$FILE_PATH.orig"

if [ ! -f "$BACKUP_PATH" ]; then
    cp "$FILE_PATH" "$BACKUP_PATH"
    echo "Original file backed up to $BACKUP_PATH"
fi

# Reset the file to original state
cp "$BACKUP_PATH" "$FILE_PATH"
echo "Reset file to original state"

# Run the resolver with Claude
echo "\nTesting with Claude provider"
export RIZZLER_PROVIDER="claude"
cargo run --bin resolve_conflicts -- "$FILE_PATH"

# Check if the file still has conflict markers
if grep -q "<<<<<<< HEAD" "$FILE_PATH" || grep -q "=======" "$FILE_PATH" || grep -q ">>>>>>>" "$FILE_PATH"; then
    echo "ERROR: Claude resolution failed - file still contains conflict markers"
    # Restore the file
    cp "$BACKUP_PATH" "$FILE_PATH"
else
    echo "SUCCESS: Claude resolution successful - no conflict markers found"
    # Save the Claude result
    cp "$FILE_PATH" "$FILE_PATH.claude"
    echo "Claude result saved to $FILE_PATH.claude"
    # Reset the file for OpenAI test
    cp "$BACKUP_PATH" "$FILE_PATH"
fi

# Run the resolver with OpenAI
echo "\nTesting with OpenAI provider"
export RIZZLER_PROVIDER="openai"
cargo run --bin resolve_conflicts -- "$FILE_PATH"

# Check if the file still has conflict markers
if grep -q "<<<<<<< HEAD" "$FILE_PATH" || grep -q "=======" "$FILE_PATH" || grep -q ">>>>>>>" "$FILE_PATH"; then
    echo "ERROR: OpenAI resolution failed - file still contains conflict markers"
    # Restore the file
    cp "$BACKUP_PATH" "$FILE_PATH"
else
    echo "SUCCESS: OpenAI resolution successful - no conflict markers found"
    # Save the OpenAI result
    cp "$FILE_PATH" "$FILE_PATH.openai"
    echo "OpenAI result saved to $FILE_PATH.openai"
fi

# Restore the original file
cp "$BACKUP_PATH" "$FILE_PATH"
echo "\nRestored original file"

echo "\nTest completed. Results saved to $FILE_PATH.claude and $FILE_PATH.openai"

# Return to original directory
cd "$ORIGINAL_DIR"