#!/bin/bash

# Check if the file is provided
if [ $# -lt 1 ]; then
    echo "Usage: $0 <file_path>"
    exit 1
fi

FILE_PATH="$1"

# Check if the file exists
if [ ! -f "$FILE_PATH" ]; then
    echo "Error: File not found: $FILE_PATH"
    exit 1
fi

# Function to check for conflict markers
check_conflicts() {
    local file=$1
    if grep -q "<<<<<<< HEAD" "$file" || \
       grep -q "=======" "$file" || \
       grep -q ">>>>>>>" "$file"; then
        return 0  # Conflicts found
    else
        return 1  # No conflicts found
    fi
}

# Check if the file has conflict markers
if check_conflicts "$FILE_PATH"; then
    echo "FAIL: File still contains merge conflict markers"
    
    # Show marker counts
    START_COUNT=$(grep -c "<<<<<<< HEAD" "$FILE_PATH")
    MID_COUNT=$(grep -c "=======" "$FILE_PATH")
    END_COUNT=$(grep -c ">>>>>>>" "$FILE_PATH")
    
    echo "Found $START_COUNT start markers (<<<<<<< HEAD)"
    echo "Found $MID_COUNT middle markers (=======)"
    echo "Found $END_COUNT end markers (>>>>>>>)"
    
    # Show line numbers of markers
    echo "
Conflict markers at these lines:"
    grep -n "<<<<<<< HEAD\|=======\|>>>>>>>" "$FILE_PATH" | sort -n
    
    exit 1
else
    echo "SUCCESS: No conflict markers found in $FILE_PATH"
    exit 0
fi