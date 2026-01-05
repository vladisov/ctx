# ctx

**Repeatable context for LLM workflows**

`ctx` is a developer tool that makes LLM workflows repeatable and manageable by treating context as a first-class object.

## What is ctx?

`ctx` solves a critical problem in LLM-assisted development: **context reproducibility**. When working with AI coding agents, you need to:

- Curate exactly what the model sees
- Preview token usage before sending
- Reproduce the same context later
- Share context setups across team members
- Version your context like you version code

`ctx` provides:

- **Context Packs**: Named bundles of files, docs, diffs, and rules
- **Deterministic Rendering**: Same pack → same output, always
- **Snapshots**: Immutable versioned payloads for reproducibility
- **MCP Integration**: Expose packs to coding agents (Claude Code, etc.)

## Quick Start

### Installation

```bash
# From source
cargo install --path crates/ctx-cli

# Or build locally
cargo build --release
```

### Basic Usage

```bash
# Create a pack
ctx pack create my-feature

# Add content
ctx pack add my-feature file:src/auth.rs
ctx pack add my-feature 'glob:tests/**/*.rs'
ctx pack add my-feature 'git:diff --base=main'

# Preview before sending to LLM
ctx pack preview my-feature --tokens

# Create immutable snapshot
ctx pack snapshot my-feature --label "v1.0"

# Serve to coding agents via MCP
ctx mcp --port 17373
```

## Use Cases

### Style Guide Pack
```bash
ctx pack create style-backend
ctx pack add style-backend 'text:Use async/await, prefer Result<T> over unwrap'
ctx pack add style-backend file:docs/CODING_STANDARDS.md
```

### Repository Context Pack
```bash
ctx pack create repo-auth
ctx pack add repo-auth 'glob:src/auth/**/*.rs'
ctx pack add repo-auth file:src/lib.rs
ctx pack add repo-auth 'git:diff --base=main'
```

### Documentation Pack
```bash
ctx pack create api-docs
ctx pack add api-docs 'md_dir:docs/api' --recursive
```

## MCP Server

Expose packs to MCP-compatible agents:

```json
{
  "tool": "ctx_packs_preview",
  "arguments": {
    "packs": ["style-guide", "repo-context"],
    "show_payload": true
  }
}
```

## Configuration

Config file: `~/.ctx/config.toml` (auto-created on first run)

```toml
budget_tokens = 128000

[denylist]
patterns = [
  "**/.env*",
  "**/.aws/**",
  "**/secrets/**",
  "**/*_rsa",
  "**/*.key",
  "**/*.pem"
]

[mcp]
host = "127.0.0.1"
port = 17373
read_only = false
```

## Security

- **Redaction**: Automatic detection and redaction of secrets (API keys, tokens, private keys)
- **Denylist**: Blocks sensitive files by default (.env, credentials, etc.)
- **Preview**: Always review what's being sent before rendering

## Development

### Prerequisites

- Rust 1.75+
- SQLite 3.x
- Git

### Setup

```bash
# Clone and build
git clone <repo>
cd ctx
cargo build

# Run tests
make test
# or manually:
cargo test                                         # Unit tests
CTX=./target/release/ctx ./tests/integration_test.sh  # Integration tests

# Install locally
cargo install --path crates/ctx-cli
```

### Project Structure

```
ctx/
├── crates/
│   ├── ctx-cli/          # CLI binary
│   ├── ctx-core/         # Domain models and core logic
│   ├── ctx-storage/      # SQLite + blob storage
│   ├── ctx-sources/      # Source handlers (file, git, glob)
│   ├── ctx-security/     # Redaction engine
│   ├── ctx-tokens/       # Token estimation
│   ├── ctx-engine/       # Render orchestration
│   ├── ctx-config/       # Configuration management
│   └── ctx-mcp/          # MCP server
```

### Development Commands

```bash
# Build and test
make build            # Debug build
make release          # Optimized build
make test             # All tests
make test-unit        # Unit tests only
make test-integration # Integration tests only
make lint             # Run clippy
make fmt              # Format code

# Run locally
cargo run -- pack create demo
cargo run -- pack add demo file:README.md
cargo run -- pack preview demo --show-payload
```

### Debugging

```bash
# Enable logging
RUST_LOG=debug cargo run -- pack list

# Use isolated test directory
CTX_DATA_DIR=/tmp/test-ctx ctx pack create test

# Inspect database
sqlite3 ~/.local/share/com.ctx.ctx/state.db
SELECT * FROM packs;
```

### Adding Features

#### New Source Handler

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

#### New CLI Command

```rust
// crates/ctx-cli/src/cli.rs
pub enum PackCommands {
    MyCommand { arg: String },
}

// crates/ctx-cli/src/commands/pack.rs
PackCommands::MyCommand { arg } => my_command(storage, arg).await,
```

## Architecture

**Key Design Principles**:
- **Deterministic rendering**: Same inputs → same output hash
- **Content-addressable storage**: Automatic deduplication
- **Security-first**: Redaction always on, sensitive files denied by default
- **Explicit context**: No hidden ingestion or automatic additions

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed architecture and implementation notes.

## Technology Stack

- **Language**: Rust
- **CLI**: clap
- **Database**: SQLite with sqlx
- **Async**: tokio
- **HTTP**: axum (for MCP server)
- **Token estimation**: tiktoken-rs
- **Hashing**: BLAKE3

## Roadmap

### M1: Packs + Persistence ✅
- SQLite database
- Pack CRUD operations
- Basic source handlers (file, glob, text)

### M2: Render + Snapshot ✅
- Deterministic render engine
- Token estimation and redaction
- Snapshot storage

### M3: MCP Server ✅
- JSON-RPC 2.0 server
- MCP tools integration

### M4: Hardening ✅
- Configuration system
- Denylist for sensitive files
- Git diff handler
- Comprehensive tests

## Contributing

1. Write tests for new features
2. Run `cargo fmt` and `cargo clippy`
3. Keep changes focused
4. Update documentation

See [ARCHITECTURE.md](./ARCHITECTURE.md) for architecture details.

## License

MIT OR Apache-2.0
