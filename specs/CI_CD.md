# CI/CD Pipeline Specification

## Overview

This document outlines the Continuous Integration and Continuous Deployment (CI/CD) pipeline for the Rizzler project, implemented through GitHub Actions.

## Objectives

- Ensure code quality by building and testing on multiple platforms
- Automate the release process for the trunk branch
- Provide compiled binaries for Linux, macOS, and Windows
- Make releases easily accessible through GitHub Releases

## Workflow Specification

### Build Job

The build job compiles and tests the application across different platforms:

- **Platforms:**
  - Ubuntu Linux (latest)
  - macOS (latest)
  - Windows (latest)

- **Steps:**
  1. Check out the repository
  2. Set up the Rust toolchain
  3. Cache Rust dependencies to speed up builds
  4. Build the application in release mode
  5. Run all tests
  6. Prepare platform-specific artifacts
  7. Upload artifacts for later use or release

### Release Job

The release job is conditionally executed only for the trunk branch:

- **Trigger:**
  - Only runs when code is pushed to the `trunk` branch
  - Depends on successful completion of the build job

- **Steps:**
  1. Download artifacts from all platforms
  2. Make Linux and macOS binaries executable
  3. Update version in Cargo.toml (incrementing patch version)
  4. Commit and push the version bump back to the repository
  5. Create a GitHub Release with all artifacts attached

## Artifacts

The following artifacts are produced by the CI/CD pipeline:

- `rizzler-linux` - Linux x86_64 binary
- `rizzler-macos` - macOS x86_64 binary  
- `rizzler-windows.exe` - Windows x86_64 executable

## Release Versioning

Releases use semantic versioning maintained automatically in two ways:

1. **Cargo.toml Versioning:**
   - The patch version is automatically incremented on every trunk build
   - Format: `MAJOR.MINOR.PATCH` (e.g., `0.1.2`)
   - The updated version is committed back to the repository
   - This is used as the official version of the application

2. **Release Tag Format:**
   - GitHub releases are tagged with the updated Cargo.toml version
   - Format: `v{MAJOR.MINOR.PATCH}` (e.g., `v0.1.2`)

This ensures proper version tracking for both the code and the released artifacts.

## Implementation

The CI/CD pipeline is implemented in `.github/workflows/build.yml` and can be tested using the workflow dispatch trigger in `.github/workflows/test-workflow.yml`.

## Future Improvements

- Add code coverage reporting
- Implement semantic versioning for major releases
- Add integration testing in the CI pipeline
- Include signing for macOS and Windows binaries 