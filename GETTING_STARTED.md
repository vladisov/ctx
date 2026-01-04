# Getting Started with ctx Development

This guide helps you set up the development environment and start contributing to `ctx`.

## Prerequisites

- Rust 1.75 or later
- SQLite 3.x (usually pre-installed on macOS/Linux)
- Git

## Initial Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Install Development Tools

```bash
# Faster test runner
cargo install cargo-nextest

# Auto-rebuild on file changes
cargo install cargo-watch

# Security audits
cargo install cargo-audit

# Dependency/license checks
cargo install cargo-deny

# Formatting and linting (usually included with rustup)
rustup component add clippy rustfmt
```

### 3. Clone and Build

```bash
cd ctx
cargo build
```

This will:
- Download all dependencies
- Build all workspace crates
- Create debug binary at `target/debug/ctx`

### 4. Run Tests

```bash
# All tests
cargo test

# Faster with nextest
cargo nextest run

# Watch mode (auto-run on changes)
cargo watch -x test
```

## Project Structure Overview

```
ctx/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── ctx-cli/                  # Binary crate (main entry point)
│   │   └── src/
│   │       ├── main.rs           # CLI entry
│   │       ├── commands/         # Command handlers
│   │       └── cli.rs            # Clap definitions
│   │
│   ├── ctx-core/                 # Core domain logic
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pack.rs           # Pack model
│   │       ├── artifact.rs       # Artifact model
│   │       ├── render.rs         # Render engine (CRITICAL)
│   │       └── snapshot.rs       # Snapshot model
│   │
│   ├── ctx-storage/              # Database & blob storage
│   │   └── src/
│   │       ├── db.rs             # SQLite operations
│   │       ├── blob.rs           # Blob store
│   │       └── migrations/       # SQL migrations
│   │
│   ├── ctx-sources/              # Source handlers
│   │   └── src/
│   │       ├── handler.rs        # SourceHandler trait
│   │       ├── file.rs           # File handler
│   │       ├── collection.rs     # Glob, md_dir handlers
│   │       └── git.rs            # Git diff handler
│   │
│   ├── ctx-security/             # Security features
│   │   └── src/
│   │       ├── redactor.rs       # Secret redaction
│   │       └── denylist.rs       # Path denylist
│   │
│   ├── ctx-tokens/               # Token estimation
│   │   └── src/
│   │       └── estimator.rs      # tiktoken wrapper
│   │
│   └── ctx-mcp/                  # MCP server
│       └── src/
│           ├── server.rs         # Axum HTTP server
│           ├── protocol.rs       # JSON-RPC 2.0
│           └── tools.rs          # MCP tools
│
└── tests/                        # Integration tests
    └── integration/
```

## Development Workflow

### Building

```bash
# Debug build (fast, unoptimized)
cargo build

# Release build (slow, optimized)
cargo build --release

# Build specific crate
cargo build -p ctx-core

# Check without building (faster)
cargo check
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p ctx-core

# Run specific test
cargo test test_render_determinism

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration

# Watch mode
cargo watch -x test
```

### Linting & Formatting

```bash
# Format all code
cargo fmt

# Check formatting without changing
cargo fmt -- --check

# Run clippy lints
cargo clippy

# Clippy with all warnings as errors
cargo clippy --all-targets -- -D warnings
```

### Running the CLI

```bash
# Debug build
cargo run -- pack create test

# Release build
cargo run --release -- pack create test

# Or build once and run directly
cargo build --release
./target/release/ctx pack create test
```

## Development Milestones

Follow the milestones in order (see [TECHNICAL_PLAN.md](./TECHNICAL_PLAN.md)):

### M1: Packs + Persistence (Current)

**Goal**: Basic CRUD for packs and artifacts

**What to build**:
1. Set up workspace structure
2. Implement `ctx-storage` (SQLite + blobs)
3. Implement `ctx-core` (domain models)
4. Implement basic source handlers in `ctx-sources`
5. Implement CLI commands in `ctx-cli`

**How to start**:
```bash
# Create crate structure
mkdir -p crates/ctx-{cli,core,storage,sources,security,tokens,mcp}/src

# Create lib.rs for each library crate
touch crates/ctx-{core,storage,sources,security,tokens,mcp}/src/lib.rs

# Create main.rs for binary crate
touch crates/ctx-cli/src/main.rs

# Add Cargo.toml for each crate (see examples below)
```

**Example crate Cargo.toml**:

```toml
# crates/ctx-core/Cargo.toml
[package]
name = "ctx-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
time = { workspace = true }
uuid = { workspace = true }
blake3 = { workspace = true }
```

**Acceptance criteria**: See M1 section in TECHNICAL_PLAN.md

### M2: Render + Snapshot

**What to build**:
- Deterministic render engine (CRITICAL)
- Token estimation
- Redaction
- Preview command

**Key focus**: Testing determinism extensively

### M3: MCP Server

**What to build**:
- Axum HTTP server
- JSON-RPC 2.0 protocol
- MCP tools

### M4: Hardening

**What to build**:
- Security features
- Additional handlers
- Documentation

## Common Tasks

### Adding a New Source Handler

1. Create handler in `crates/ctx-sources/src/`
2. Implement `SourceHandler` trait
3. Register in handler registry
4. Add tests
5. Update CLI parser

Example:
```rust
// crates/ctx-sources/src/my_handler.rs
use async_trait::async_trait;
use crate::handler::SourceHandler;

pub struct MyHandler;

#[async_trait]
impl SourceHandler for MyHandler {
    async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact> {
        // Implementation
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        // Implementation
    }

    async fn expand(&self, artifact: &Artifact) -> Result<Vec<Artifact>> {
        // Implementation
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("my:")
    }
}
```

### Adding a Database Migration

```bash
# Create migration file
touch crates/ctx-storage/src/migrations/002_add_feature.sql
```

```sql
-- 002_add_feature.sql
ALTER TABLE packs ADD COLUMN new_field TEXT;
CREATE INDEX idx_packs_new_field ON packs(new_field);
```

### Adding an MCP Tool

```rust
// In crates/ctx-mcp/src/tools.rs
async fn call_tool(server: &McpServer, params: &serde_json::Value) -> Result<serde_json::Value> {
    match tool_name {
        "ctx_my_new_tool" => {
            // Implementation
        }
        // ... existing tools
    }
}
```

## Debugging Tips

### Enable Logging

```bash
# All logs
RUST_LOG=debug cargo run -- pack create test

# Specific module
RUST_LOG=ctx_core::render=trace cargo run -- pack preview test

# Multiple modules
RUST_LOG=ctx_core=debug,sqlx=info cargo run
```

### Database Inspection

```bash
# Open database
sqlite3 ~/.ctx/state.db

# Useful queries
.tables
SELECT * FROM packs;
SELECT * FROM artifacts;
SELECT * FROM pack_items ORDER BY priority DESC, added_at ASC;
```

### Debugging Blob Storage

```bash
# List all blobs
find ~/.ctx/blobs -type f

# Check blob content
cat ~/.ctx/blobs/blake3/a3/a3f2b8c9d1...

# Verify hash
cat ~/.ctx/blobs/blake3/a3/a3f2b8c9d1... | b3sum
```

## Performance Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin ctx -- pack preview large-pack

# Benchmarks
cargo bench
```

## CI/CD

GitHub Actions will run on every push:
- `cargo test`
- `cargo clippy`
- `cargo fmt -- --check`
- `cargo audit`

Ensure these pass locally before pushing:
```bash
cargo test && cargo clippy -- -D warnings && cargo fmt -- --check
```

## Getting Help

- Read [TECHNICAL_PLAN.md](./TECHNICAL_PLAN.md) for architecture details
- Check existing tests for examples
- Review code in similar handlers/components

## Next Steps

1. **Set up environment** (steps above)
2. **Read TECHNICAL_PLAN.md** (understand architecture)
3. **Start with M1** (pick a component to implement)
4. **Write tests first** (TDD approach recommended)
5. **Submit PRs** (small, focused changes)

Happy coding!
