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

# Use test mode if Gemini API key is not available
if [ -z "$RIZZLER_GEMINI_API_KEY" ]; then
    echo "Gemini API key is not set. Using TEST_MODE=true for the test"
    export TEST_MODE=true
    export RIZZLER_GEMINI_API_KEY="test-key"
fi

# Run the resolver with Gemini
echo "\nTesting with Gemini provider"
export RIZZLER_PROVIDER="gemini"
cargo run --bin resolve_conflicts -- "$FILE_PATH"

# Check if the file still has conflict markers
if grep -q "<<<<<<< HEAD" "$FILE_PATH" || grep -q "=======" "$FILE_PATH" || grep -q ">>>>>>>" "$FILE_PATH"; then
    echo "ERROR: Gemini resolution failed - file still contains conflict markers"
    # Restore the file
    cp "$BACKUP_PATH" "$FILE_PATH"
else
    echo "SUCCESS: Gemini resolution successful - no conflict markers found"
    # Save the Gemini result
    cp "$FILE_PATH" "$FILE_PATH.gemini"
    echo "Gemini result saved to $FILE_PATH.gemini"
fi

# Restore the original file
cp "$BACKUP_PATH" "$FILE_PATH"
echo "\nRestored original file"

echo "\nTest completed. Results saved to $FILE_PATH.gemini"

# Return to original directory
cd "$ORIGINAL_DIR"