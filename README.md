# ctx

**Repeatable context for LLM workflows**

`ctx` is a developer tool that makes LLM workflows repeatable and manageable by treating context as a first-class object.

## Status

✅ **MVP Complete (M1-M4)** - Production-ready context management for LLMs

## Documentation

- **[User Guide (WALKTHROUGH.md)](./WALKTHROUGH.md)**: How to install and use `ctx`.
- **[Developer Guide (GETTING_STARTED.md)](./GETTING_STARTED.md)**: How to build and contribute.
- **[Technical Plan (TECHNICAL_PLAN.md)](./TECHNICAL_PLAN.md)**: Architecture and roadmap.

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

## MVP Features

### CLI

```bash
# Create and manage packs
ctx pack create my-feature
ctx pack add my-feature file:src/auth.rs
ctx pack add my-feature 'glob:tests/**/*.rs'
ctx pack add my-feature 'git:diff --base=main'

# Preview what will be sent to LLM
ctx pack preview my-feature --tokens
ctx pack preview my-feature --show-payload

# Create immutable snapshot
ctx pack snapshot my-feature --name "v1.0"

# Serve to coding agents via MCP
ctx mcp serve
```

### MCP Server

Expose packs to MCP-compatible agents:

```json
// Agent calls ctx.packs.render
{
  "tool": "ctx_packs_render",
  "arguments": {
    "packs": ["style-guide", "repo-context"],
    "show_payload": true
  }
}
```

## Use Cases

### Style Guide Pack
```bash
ctx pack create style-backend
ctx pack add style-backend 'text:Use async/await, prefer Result<T> over unwrap'
ctx pack add style-backend md:docs/CODING_STANDARDS.md
```

### Repository Context Pack
```bash
ctx pack create repo-auth
ctx pack add repo-auth 'glob:src/auth/**/*.rs'
ctx pack add repo-auth file:src/lib.rs
ctx pack add repo-auth 'git:diff --base=main --head=feature-branch'
```

### Documentation Pack
```bash
ctx pack create api-docs
ctx pack add api-docs 'md_dir:docs/api' --recursive
```

Then combine packs when working with your agent:
```bash
ctx pack render style-backend --with-pack repo-auth --with-pack api-docs
```

## Architecture

```
ctx/
├── crates/
│   ├── ctx-cli/          # Binary crate
│   ├── ctx-core/         # Core domain logic
│   ├── ctx-storage/      # SQLite + blob storage
│   ├── ctx-sources/      # Source handlers (file, git, glob)
│   ├── ctx-security/     # Redaction engine
│   ├── ctx-tokens/       # Token estimation
│   ├── ctx-engine/       # Render orchestration
│   ├── ctx-config/       # Configuration management
│   └── ctx-mcp/          # MCP server
```

**Key Design Principles**:
- **Deterministic rendering**: Same inputs → same output hash
- **Content-addressable storage**: Automatic deduplication
- **Security-first**: Redaction always on, sensitive files denied by default
- **Explicit context**: No hidden ingestion or automatic additions

## Technology Stack

- **Language**: Rust
- **CLI**: clap
- **Database**: SQLite with sqlx
- **Async**: tokio
- **HTTP**: axum (for MCP server)
- **Token estimation**: tiktoken-rs
- **Hashing**: BLAKE3

## Development Roadmap

### M1: Packs + Persistence ✅ (Weeks 1-2)
- SQLite database
- Pack CRUD operations
- Basic source handlers (file, glob, text)

### M2: Render + Snapshot ✅
- Deterministic render engine
- Token estimation
- Redaction
- Snapshot storage

### M3: MCP Server ✅
- JSON-RPC 2.0 server
- MCP tools (list, get, preview, snapshot)
- Integration with render engine

### M4: Hardening ✅
- Configuration system (~/.ctx/config.toml)
- Denylist for sensitive files
- Git diff handler (git:diff)
- Integration tests

## Getting Started (Developers)

```bash
# Clone and build
git clone <repo-url>
cd ctx
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path crates/ctx-cli

# Run
ctx --help
```

See [TECHNICAL_PLAN.md](./TECHNICAL_PLAN.md) for detailed implementation guide.

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

## Non-Goals (MVP)

- ❌ Code intelligence / RAG / indexing
- ❌ File editing / patch application
- ❌ Built-in model execution (`ctx run`)
- ❌ Web UI
- ❌ Remote storage / multi-user

These may come in future phases.

## License

MIT OR Apache-2.0

## Contributing

See [TECHNICAL_PLAN.md](./TECHNICAL_PLAN.md) for architecture and implementation details.
