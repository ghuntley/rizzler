# Command Line Interface

## Overview

The rizzler will use the Rust `clap` crate to implement a command-line interface for configuration, manual conflict resolution, and integration with Git.

## Command Structure

### Primary Commands

```
rizzler [SUBCOMMAND]
```

Without a subcommand, the binary acts as a Git merge driver, reading from standard input and writing to standard output according to Git's merge driver protocol.

### Subcommands

1. **setup**
   - Configure rizzler as a merge driver in Git

2. **config**
   - View and modify configuration settings

3. **resolve**
   - Manually resolve conflicts in a file

4. **version**
   - Display version information

5. **doctor**
   - Check configuration and diagnose issues

## Command Details

### Setup Command

```
rizzler setup [--global] [--local] [--extensions <EXTENSIONS>...]
```

Options:
- `--global`: Configure globally in user's .gitconfig
- `--local`: Configure only for current repository
- `--extensions`: Specify file extensions to associate with the merge driver

### Config Command

```
rizzler config [get|set|list] [KEY] [VALUE]
```

Subcommands:
- `get <KEY>`: Get the value of a specific configuration key
- `set <KEY> <VALUE>`: Set a configuration value
- `list`: List all configuration values

### Resolve Command

```
rizzler resolve <FILE> [--output <FILE>] [--provider <PROVIDER>]
```

Options:
- `--output`: Specify output file (default: stdout)
- `--provider`: Specify AI provider to use
- `--model`: Specify model to use
- `--strategy`: Resolution strategy (ai, rule-based, manual)

## Implementation with Clap

The CLI will use Clap's derive API for a type-safe, declarative command structure:

```rust
#[derive(Parser)]
#[command(name = "rizzler")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Setup(SetupArgs),
    Config(ConfigArgs),
    Resolve(ResolveArgs),
    Version,
    Doctor(DoctorArgs),
}
```

## Environment Variables

The CLI will recognize these environment variables in addition to provider-specific ones:

- `RIZZLER_CONFIG_PATH`: Override path to configuration file
- `RIZZLER_DEBUG`: Enable debug output (1=true, 0=false)
- `RIZZLER_TIMEOUT`: Default timeout in seconds