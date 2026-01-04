.PHONY: build test clean install check fmt clippy run help

# Default target
help:
	@echo "ctx - Makefile targets:"
	@echo "  build       - Build debug binary"
	@echo "  release     - Build optimized release binary"
	@echo "  test        - Run all tests"
	@echo "  check       - Quick compile check (no binary)"
	@echo "  fmt         - Format all code"
	@echo "  clippy      - Run clippy lints"
	@echo "  clean       - Remove build artifacts"
	@echo "  install     - Install to ~/.cargo/bin"
	@echo "  run         - Run with cargo (pass ARGS='...')"
	@echo "  watch       - Auto-rebuild on changes"
	@echo "  ci          - Run CI checks (test + clippy + fmt)"

build:
	cargo build

release:
	cargo build --release
	@echo "Binary at: target/release/ctx"

test:
	cargo test

check:
	cargo check --all-targets

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
	rm -rf target/

install:
	cargo install --path crates/ctx-cli

run:
	cargo run -- $(ARGS)

watch:
	cargo watch -x check -x test

# CI target - runs all checks
ci: test clippy
	cargo fmt --all -- --check

# Development helpers
dev-setup:
	@echo "Installing development tools..."
	cargo install cargo-watch
	cargo install cargo-nextest
	cargo install cargo-audit
	rustup component add clippy rustfmt

# Run with logging
run-debug:
	RUST_LOG=debug cargo run -- $(ARGS)

# Quick check during development
quick:
	cargo check

# Generate documentation
docs:
	cargo doc --no-deps --open
