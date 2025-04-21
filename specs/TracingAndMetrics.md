# Tracing and Metrics

## Overview

The git-merge-ai-resolver will use the Rust `tracing` crate for structured logging, diagnostics, and metrics collection.

## Tracing Architecture

### Core Components

1. **Spans and Events**
   - Hierarchical spans will track resolution lifecycle stages
   - Events within spans will capture important points in execution
   - Structured fields will provide context for debugging and analysis

2. **Subscribers**
   - Console logging for interactive use
   - File logging for persistent records
   - Optional OpenTelemetry export for monitoring systems

3. **Filtering**
   - Environment-variable based filter configuration
   - Per-module granularity for logging levels

## Implementation Details

### Dependency Structure

```
tracing          - Core tracing framework
tracing-subscriber - Subscriber implementations
tracing-appender   - Log file rotation and management
tracing-opentelemetry - Optional OpenTelemetry integration
```

### Traced Operations

1. **Configuration Loading**
   - Track gitconfig parsing
   - Record effective configuration

2. **Merge Driver Execution**
   - Entry/exit of driver execution
   - Timing of overall resolution process

3. **AI Provider API Calls**
   - Request initiation
   - Response timing and status
   - Token usage metrics (masked for privacy)

4. **Conflict Parsing**
   - Number and size of conflict regions
   - Parse success/failure

5. **Resolution Decisions**
   - Strategy selection
   - Resolution attempt status
   - Fallback mechanisms triggered

### Metrics Collection

1. **Performance Metrics**
   - Resolution time per file
   - AI provider response time
   - Parse time for conflicts

2. **Success Metrics**
   - Resolution success rate
   - Fallback frequency
   - Error types and frequencies

3. **Resource Usage**
   - Memory consumption
   - Token usage by AI model
   - API calls made

## Configuration

### Environment Variables

- `GIT_MERGE_LOG_LEVEL`: Overall logging level (error, warn, info, debug, trace)
- `GIT_MERGE_LOG_FILE`: Path to log file (if not provided, logs to stderr only)
- `GIT_MERGE_LOG_FORMAT`: Format for logs (compact, pretty, json)
- `GIT_MERGE_METRICS_ENABLED`: Enable/disable metrics collection