# Installation Guide

## Prerequisites

### Install Rust

The ctx project is written in Rust. You need to install Rust to build and run it.

#### macOS/Linux

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the on-screen instructions. After installation, restart your terminal or run:

```bash
source $HOME/.cargo/env
```

#### Verify Installation

```bash
rustc --version
cargo --version
```

You should see version 1.75 or later.

## Building ctx

### 1. Clone the Repository

If you haven't already:

```bash
git clone <your-repo-url>
cd little_coding_thingy
```

### 2. Build the Project

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (optimized, for production use)
cargo build --release
```

The binary will be available at:
- Debug: `./target/debug/ctx`
- Release: `./target/release/ctx`

### 3. Run Tests

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_pack_lifecycle
```

### 4. Install Globally (Optional)

To install `ctx` globally so you can run it from anywhere:

```bash
cargo install --path crates/ctx-cli
```

This will install the binary to `~/.cargo/bin/ctx`.

Make sure `~/.cargo/bin` is in your PATH.

## Quick Start

After building, try these commands:

```bash
# Using the binary directly
./target/release/ctx pack create my-first-pack

# Or if installed globally
ctx pack create my-first-pack

# Add some content
ctx pack add my-first-pack file:./README.md
ctx pack add my-first-pack 'text:This is a test'

# View the pack
ctx pack show my-first-pack

# List all packs
ctx pack list
```

## Troubleshooting

### SQLite Errors

If you see SQLite-related errors during build:

**macOS**:
```bash
brew install sqlite
```

**Ubuntu/Debian**:
```bash
sudo apt-get install libsqlite3-dev
```

### Compilation Errors

If you encounter compilation errors:

1. Update Rust to the latest version:
   ```bash
   rustup update
   ```

2. Clean the build cache:
   ```bash
   cargo clean
   cargo build --release
   ```

### Performance Issues

If the debug build is too slow, always use the release build:

```bash
cargo build --release
./target/release/ctx --help
```

## Development Setup

For development, you may want to install additional tools:

```bash
# Auto-rebuild on file changes
cargo install cargo-watch

# Faster test runner
cargo install cargo-nextest

# Code coverage
cargo install cargo-tarpaulin
```

### Development Workflow

```bash
# Watch mode: auto-rebuild on changes
cargo watch -x check -x test

# Run tests with nextest (faster)
cargo nextest run

# Format code
cargo fmt

# Lint code
cargo clippy --all-targets
```
