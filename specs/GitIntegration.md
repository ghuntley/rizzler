# Git Integration

## Overview

The rizzler integrates with Git as a custom merge driver, allowing it to automatically resolve conflicts during merge operations.

## Git Merge Driver Interface

When invoked as a Git merge driver, the tool will accept the standard Git merge driver arguments:

```
rizzler %O %A %B %P
```

Where:
- `%O`: Path to the ancestor's version of the file
- `%A`: Path to the current version of the file
- `%B`: Path to the other branches' version of the file
- `%P`: Path to the file with conflict markers

## Integration Process

1. **Configuration Installation**
   - Global or per-repository configuration in .gitconfig
   - File extension associations via gitattributes

2. **Invocation by Git**
   - Git detects a merge conflict in a file with a configured extension
   - Git calls our merge driver with the appropriate parameters

3. **Resolution Process**
   - Merge driver parses conflict information
   - AI resolution is applied to the conflicts
   - Resolved file is written back to the filesystem
   - Success/failure status is returned to Git

## Git Configuration Mechanism

### Gitconfig

The setup process adds the following to `.gitconfig`:

```
[merge "rizzler"]
    name = AI-powered Git merge conflict resolver
    driver = rizzler %O %A %B %P
    trustExitCode = true
```

### Gitattributes

File associations are configured in `.gitattributes`:

```
*.js merge=rizzler
*.py merge=rizzler
*.rs merge=rizzler
# etc.
```

## Exit Codes

- `0`: Success - conflicts resolved
- Non-zero: Failure - manual resolution needed

## Data Flow from Git's Perspective

1. Git identifies a conflict during merge/rebase/pull
2. Git looks up the merge driver for the file's extension
3. Git invokes our driver with paths to conflicting versions
4. Our driver resolves conflicts and writes results to the filesystem
5. Git continues with the merge process (commit, etc.)