# Getting Started with ctx Development

Quick guide for developers contributing to ctx.

---

## Prerequisites

- Rust 1.75+
- SQLite 3.x (usually pre-installed)
- Git

## Quick Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install tools
cargo install cargo-watch cargo-nextest

# Clone and build
cd ctx
cargo build

# Run tests
cargo test
```

---

## Project Structure

```
ctx/
├── crates/
│   ├── ctx-cli/          # Binary (CLI entry point)
│   ├── ctx-core/         # Domain models
│   ├── ctx-storage/      # SQLite + blob storage
│   ├── ctx-sources/      # Source handlers
│   ├── ctx-security/     # Redaction (M2)
│   ├── ctx-tokens/       # Token estimation (M2)
│   └── ctx-mcp/          # MCP server (M3)
│
├── M1_COMPLETE.md        # M1 status + improvements
├── TECHNICAL_PLAN.md     # Full implementation plan
└── README.md             # Project overview
```

---

## Development Workflow

### Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo check              # Fast check (no binary)
cargo clippy             # Linting
cargo fmt                # Formatting
```

### Testing

```bash
cargo test                    # All tests
cargo nextest run             # Faster test runner
cargo test test_pack_lifecycle  # Specific test
cargo test --test integration # Integration tests only
```

### Development Loop

```bash
# Auto-rebuild on changes
cargo watch -x check -x test

# Or use Makefile
make watch
```

---

## Try It Out

```bash
# Build and run
cargo run -- pack create demo

# Add artifacts
cargo run -- pack add demo README.md
cargo run -- pack add demo 'text:Use Rust idioms'

# View pack
cargo run -- pack show demo
cargo run -- pack list
```

---

## Current Status (M1)

✅ **Implemented**:
- Pack CRUD operations
- Artifact management (file, text, collections)
- SQLite storage with migrations
- Blob storage (content-addressable)
- CLI commands

⏳ **TODO (M2-M4)**:
- Token estimation (M2)
- Redaction (M2)
- Rendering (M2)
- Snapshots (M2)
- MCP server (M3)

See [M1_COMPLETE.md](./M1_COMPLETE.md) for details.

---

## Making Changes

### Adding a Source Handler

```rust
// 1. Create handler in crates/ctx-sources/src/my_handler.rs
pub struct MyHandler;

#[async_trait]
impl SourceHandler for MyHandler {
    async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact> {
        // Parse URI
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        // Load content
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("my:")
    }
}

// 2. Register in handler.rs
registry.register(Arc::new(MyHandler));
```

### Adding a CLI Command

```rust
// 1. Add to crates/ctx-cli/src/cli.rs
pub enum PackCommands {
    MyCommand { arg: String },
}

// 2. Handle in crates/ctx-cli/src/commands/pack.rs
PackCommands::MyCommand { arg } => my_command(storage, arg).await,
```

---

## Debugging

### Enable Logging

```bash
# All logs
RUST_LOG=debug cargo run -- pack list

# Specific module
RUST_LOG=ctx_storage=trace cargo run

# Multiple modules
RUST_LOG=ctx_storage=debug,sqlx=info cargo run
```

### Inspect Database

```bash
sqlite3 ~/.local/share/ctx/ctx/state.db

# Useful queries
.tables
.schema packs
SELECT * FROM packs;
SELECT * FROM artifacts;
SELECT * FROM _migrations;
```

### Check Blob Storage

```bash
# List blobs
find ~/.local/share/ctx/ctx/blobs -type f

# Verify hash
cat ~/.local/share/ctx/ctx/blobs/blake3/a3/a3f2... | b3sum
```

---

## Common Tasks

### Reset Database

```bash
rm -rf ~/.local/share/ctx/ctx/
cargo run -- pack create fresh-start
```

### Run CI Checks Locally

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

### Profile Performance

```bash
cargo install flamegraph
cargo flamegraph --bin ctx -- pack list
```

---

## Contributing Guidelines

1. **Write tests** for new features
2. **Run `cargo fmt`** before committing
3. **Fix clippy warnings**: `cargo clippy`
4. **Update docs** if changing APIs
5. **Keep PRs focused** (one feature per PR)

---

## Useful Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [SQLx Documentation](https://docs.rs/sqlx)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Project Technical Plan](./TECHNICAL_PLAN.md)

---

## Getting Help

- Check [M1_COMPLETE.md](./M1_COMPLETE.md) for status
- Review [TECHNICAL_PLAN.md](./TECHNICAL_PLAN.md) for architecture
- Look at existing code for examples
- Check tests for usage patterns

Happy coding! 🦀
