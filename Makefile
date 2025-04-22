# Makefile for rizzler - AI-powered Git merge conflict resolver

# Configuration
CARGO := cargo
BIN_NAME := rizzler
RELEASE_TARGET_DIR := target/release
DEBUG_TARGET_DIR := target/debug

# Platform-specific settings
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
	BINARY := $(BIN_NAME)-linux
endif
ifeq ($(UNAME_S),Darwin)
	BINARY := $(BIN_NAME)-macos
endif
ifeq ($(findstring MINGW,$(UNAME_S)),MINGW)
	BINARY := $(BIN_NAME)-windows.exe
endif

# Default target
.PHONY: all
all: check test build

# Build targets
.PHONY: build build-release
build:
	$(CARGO) build

build-release:
	$(CARGO) build --release

# Testing targets
.PHONY: test test-unit test-integration test-all test-coverage test-proptest
test:
	$(CARGO) test

test-unit:
	$(CARGO) test --lib

test-integration:
	$(CARGO) test --test '*' -- --ignored

test-all: test test-integration

# Property-based testing specifically
test-proptest:
	$(CARGO) test -- --nocapture proptest

# Test coverage using cargo-tarpaulin
test-coverage:
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { \
		echo "cargo-tarpaulin is not installed. Installing..."; \
		cargo install cargo-tarpaulin; \
	}
	cargo tarpaulin --out Xml --output-dir target/coverage

# Benchmarking
.PHONY: bench
bench:
	@command -v cargo-criterion >/dev/null 2>&1 || { \
		echo "cargo-criterion is not installed. Installing..."; \
		cargo install cargo-criterion; \
	}
	cargo criterion

# Linting and formatting
.PHONY: check fmt lint clippy
check:
	$(CARGO) check

fmt:
	$(CARGO) fmt

lint: fmt clippy

clippy:
	$(CARGO) clippy -- -D warnings

# Documentation
.PHONY: doc
doc:
	$(CARGO) doc --no-deps

# Installation
.PHONY: install install-release
install:
	$(CARGO) install --path .

install-release:
	$(CARGO) install --path . --release

# Spec update - keep the specs in sync with the implementation
.PHONY: update-specs
update-specs:
	@echo "Updating specs to match the current implementation..."
	@for spec in specs/*.md; do \
		echo "Checking $$spec..."; \
		git diff --exit-code $$spec || echo "$$spec needs to be updated"; \
	done

# Clean the project
.PHONY: clean
clean:
	$(CARGO) clean

# Run the project
.PHONY: run
run:
	$(CARGO) run

# Package for release
.PHONY: package
package: build-release
	@mkdir -p dist
	@cp $(RELEASE_TARGET_DIR)/$(BIN_NAME) dist/$(BINARY)
	@echo "Created release package at dist/$(BINARY)"

# Help target
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  all            - Check, test, and build the project (default)"
	@echo "  build          - Build the project in debug mode"
	@echo "  build-release  - Build the project in release mode"
	@echo "  test           - Run tests"
	@echo "  test-unit      - Run unit tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  test-all       - Run all tests including integration tests"
	@echo "  test-proptest  - Run property-based tests specifically"
	@echo "  test-coverage  - Run tests with coverage reporting using cargo-tarpaulin"
	@echo "  bench          - Run benchmarks using criterion"
	@echo "  check          - Check project for errors"
	@echo "  fmt            - Format source code"
	@echo "  lint           - Run linters (fmt and clippy)"
	@echo "  clippy         - Run clippy linter"
	@echo "  doc            - Generate documentation"
	@echo "  install        - Install the project locally"
	@echo "  install-release - Install the project locally (release version)"
	@echo "  update-specs   - Check if specs need to be updated"
	@echo "  clean          - Clean build artifacts"
	@echo "  run            - Run the project"
	@echo "  package        - Create release package"
	@echo "  help           - Display this help message" 