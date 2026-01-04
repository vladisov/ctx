# Getting Started (Contributors)

Quick guide for developers working on ctx.

## Prerequisites

- Rust 1.75+
- SQLite 3.x
- Git

## Setup

```bash
# Clone and build
cd ctx
cargo build

# Run tests
cargo test

# Install locally
cargo install --path crates/ctx-cli
```

## Project Structure

```
ctx/
├── crates/
│   ├── ctx-cli/      # CLI binary
│   ├── ctx-core/     # Domain models
│   ├── ctx-storage/  # SQLite + blobs
│   ├── ctx-sources/  # Source handlers
│   ├── ctx-security/ # Redaction
│   ├── ctx-tokens/   # Token estimation
│   ├── ctx-engine/   # Render engine
│   └── ctx-mcp/      # MCP server
```

## Development

```bash
# Build and test
cargo build
cargo test
cargo clippy
cargo fmt

# Run locally
cargo run -- pack create demo
cargo run -- pack add demo README.md
cargo run -- pack preview demo --show-payload
```

## Debugging

```bash
# Enable logging
RUST_LOG=debug cargo run -- pack list

# Inspect database
sqlite3 ~/.local/share/ctx/ctx/state.db
SELECT * FROM packs;

# Reset state
rm -rf ~/.local/share/ctx/ctx/
```

## Adding Features

### New Source Handler

```rust
// crates/ctx-sources/src/my_handler.rs
pub struct MyHandler;

#[async_trait]
impl SourceHandler for MyHandler {
    async fn parse(&self, uri: &str, _: SourceOptions) -> Result<Artifact> {
        // Parse URI into artifact
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        // Load content
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("myscheme:")
    }
}
```

Register in `handler.rs`:
```rust
registry.register(Arc::new(MyHandler));
```

### New CLI Command

```rust
// crates/ctx-cli/src/cli.rs
pub enum PackCommands {
    MyCommand { arg: String },
}

// crates/ctx-cli/src/commands/pack.rs
PackCommands::MyCommand { arg } => my_command(storage, arg).await,
```

## Contributing

1. Write tests
2. Run `cargo fmt` and `cargo clippy`
3. Keep changes focused
4. Update docs if needed

See [TECHNICAL_PLAN.md](./TECHNICAL_PLAN.md) for architecture details.
