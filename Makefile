# Makefile for ctx

.PHONY: help build release test test-unit test-integration clean install fmt lint

help:
	@echo "ctx - Context management for LLMs"
	@echo ""
	@echo "Available targets:"
	@echo "  build             - Build debug binary"
	@echo "  release           - Build release binary (optimized)"
	@echo "  test              - Run all tests"
	@echo "  test-unit         - Run unit tests only"
	@echo "  test-integration  - Run integration tests"
	@echo "  clean             - Clean build artifacts"
	@echo "  install           - Install ctx binary"
	@echo "  fmt               - Format code"
	@echo "  lint              - Run clippy"

build:
	cargo build

release:
	cargo build --release

test: test-unit test-integration

test-unit:
	cargo test

test-integration: release
	@echo "Running integration tests..."
	@CTX=./target/release/ctx ./tests/integration_test.sh

clean:
	cargo clean

install:
	cargo install --path crates/ctx-cli

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

.DEFAULT_GOAL := help
