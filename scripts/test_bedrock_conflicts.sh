#!/bin/bash

# Save current working directory
ORIGINAL_DIR=$(pwd)

# Source profile to get AWS keys
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

# Use test mode if AWS credentials are not available
if [ -z "$AWS_ACCESS_KEY_ID" ] || [ -z "$AWS_SECRET_ACCESS_KEY" ]; then
    echo "AWS credentials are not set. Using TEST_MODE=true for the test"
    export TEST_MODE=true
    export AWS_ACCESS_KEY_ID="test-key"
    export AWS_SECRET_ACCESS_KEY="test-secret"
fi

# Set AWS region if not already set
if [ -z "$AWS_REGION" ]; then
    export AWS_REGION="us-east-1"
    echo "Set AWS_REGION to $AWS_REGION"
fi

# Run the resolver with Bedrock
echo "\nTesting with Bedrock provider"
export RIZZLER_PROVIDER="bedrock"
cargo run --bin resolve_conflicts -- "$FILE_PATH"

# Check if the file still has conflict markers
if grep -q "<<<<<<< HEAD" "$FILE_PATH" || grep -q "=======" "$FILE_PATH" || grep -q ">>>>>>>" "$FILE_PATH"; then
    echo "ERROR: Bedrock resolution failed - file still contains conflict markers"
    # Restore the file
    cp "$BACKUP_PATH" "$FILE_PATH"
else
    echo "SUCCESS: Bedrock resolution successful - no conflict markers found"
    # Save the Bedrock result
    cp "$FILE_PATH" "$FILE_PATH.bedrock"
    echo "Bedrock result saved to $FILE_PATH.bedrock"
fi

# Restore the original file
cp "$BACKUP_PATH" "$FILE_PATH"
echo "\nRestored original file"

echo "\nTest completed. Results saved to $FILE_PATH.bedrock"

# Return to original directory
cd "$ORIGINAL_DIR"