# ctx - Quick Start

## Initial Setup

```bash
# 1. Ensure Rust is installed
rustup --version

# 2. Build the project
cargo build

# 3. Run tests
cargo test

# 4. Install development tools
make dev-setup
```

## Development Commands

```bash
# Build
make build              # Debug build
make release            # Release build

# Testing
make test               # Run all tests
cargo nextest run       # Faster test runner

# Linting & Formatting
make fmt                # Format code
make clippy             # Run lints
make ci                 # Run all CI checks

# Development
make watch              # Auto-rebuild on changes
make run ARGS="--help"  # Run CLI
make run-debug ARGS="pack list"  # Run with debug logging
```

## Project Structure

```
ctx/
├── crates/
│   ├── ctx-cli/          → Binary (main.rs)
│   ├── ctx-core/         → Domain models + render engine
│   ├── ctx-storage/      → SQLite + blob storage
│   ├── ctx-sources/      → Source handlers
│   ├── ctx-security/     → Redaction + denylist
│   ├── ctx-tokens/       → Token estimation
│   └── ctx-mcp/          → MCP server
├── TECHNICAL_PLAN.md     → Full implementation plan
├── GETTING_STARTED.md    → Detailed dev guide
└── README.md             → Project overview
```

## Implementation Order (MVP)

### M1: Packs + Persistence ✅ Start Here
**Goal**: Basic pack CRUD operations

**Tasks**:
1. Implement `ctx-storage`:
   - `db.rs` - SQLite operations with sqlx
   - `blob.rs` - Content-addressable blob storage

2. Implement `ctx-sources`:
   - `handler.rs` - SourceHandler trait ✓ (done)
   - `file.rs` - File handler (file:path, file:path#Lx-Ly)
   - `collection.rs` - md_dir and glob handlers
   - `text.rs` - Inline text handler

3. Implement `ctx-cli` commands:
   - `pack create <name>`
   - `pack list`
   - `pack show <pack>`
   - `pack add <pack> <source>`
   - `pack remove <pack> <artifact-id>`

**Acceptance**:
```bash
ctx pack create test
ctx pack add test file:README.md
ctx pack list
ctx pack show test
```

### M2: Render + Snapshot
**Goal**: Deterministic rendering + snapshots

**Tasks**:
1. `ctx-tokens` - Token estimator with tiktoken
2. `ctx-security` - Redaction engine
3. `ctx-core/render.rs` - **CRITICAL** deterministic render engine
4. `ctx-cli` - Preview and snapshot commands

**Acceptance**:
```bash
ctx pack preview test --tokens
ctx pack preview test --show-payload
ctx pack snapshot test --name "v1.0"
```

### M3: MCP Server
**Goal**: Expose packs via MCP

**Tasks**:
1. `ctx-mcp` - Axum HTTP server + JSON-RPC
2. Implement MCP tools
3. `ctx mcp serve` command

**Acceptance**:
```bash
ctx mcp serve --port 17373
# Test with curl or MCP client
```

### M4: Hardening
**Goal**: Security + additional features

**Tasks**:
1. Denylist implementation
2. Git diff handler
3. Command output handler
4. Configuration system
5. Documentation

## Tips

- **Start with M1** - Get pack CRUD working first
- **Write tests first** - TDD approach recommended
- **Focus on determinism** - Critical for M2 render engine
- **Use the Makefile** - `make help` for all targets
- **Check logs** - `RUST_LOG=debug make run ARGS="..."`

## Getting Help

- Read [TECHNICAL_PLAN.md](./TECHNICAL_PLAN.md) for architecture
- Read [GETTING_STARTED.md](./GETTING_STARTED.md) for details
- Look at TODO comments in source files
- Check existing tests for examples

## First Contribution Ideas

Pick one to start:

1. **Easy**: Implement `TextHandler` in `ctx-sources/src/text.rs`
2. **Medium**: Implement `BlobStore` in `ctx-storage/src/blob.rs`
3. **Medium**: Implement `FileHandler` in `ctx-sources/src/file.rs`
4. **Hard**: Implement database operations in `ctx-storage/src/db.rs`
5. **Hard**: Start render engine in `ctx-core/src/render.rs`

Happy coding! 🦀
