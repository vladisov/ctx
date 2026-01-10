# Changelog

All notable changes and milestones for the ctx project.

---

## M4: Hardening (2026-01-04) ✅

**Summary**: Configuration system, security hardening with denylist, git diff support, and integration testing.

### Features
- Configuration file at `~/.ctx/config.toml` (auto-created)
- Denylist patterns to block sensitive files (.env, .aws, secrets, keys, etc.)
- Git diff source handler: `git:diff [--base=REF] [--head=REF]`
- Integration test suite (10 tests covering core functionality)
- Config-based defaults for token budget and MCP settings

### Implementation
- New crate: `ctx-config` for TOML configuration management
- Denylist module in `ctx-sources` with glob pattern matching
- Git handler using command-line `git diff`
- Denylist validation during artifact addition
- Config defaults: 128K tokens, deny patterns for common secrets

### Testing
- Integration test script: `tests/integration_test.sh`
- Tests cover: pack creation, artifacts, denylist, preview, determinism, ctx.toml sync/save

---

## M3: MCP Server (2026-01-04) ✅

**Summary**: JSON-RPC 2.0 server for exposing ctx functionality to MCP-compatible AI agents.

### Features
- `ctx mcp --port 17373 --host 127.0.0.1 --read-only`
- JSON-RPC 2.0 protocol implementation
- MCP tools: `ctx_packs_list`, `ctx_packs_get`, `ctx_packs_preview`, `ctx_packs_create`, `ctx_packs_add_artifact`, `ctx_packs_delete`
- Read-only mode for safety

### Implementation
- New crate: `ctx-mcp`
- Axum-based HTTP server
- Integration with `ctx-engine` for rendering

---

## M2: Render Engine (2026-01-04) ✅

**Summary**: Deterministic rendering engine with token budgeting and redaction.

### Features
- `ctx pack preview <pack> [--tokens] [--redactions] [--show-payload]`
- Deterministic rendering (same inputs → same hash)
- Token estimation using tiktoken (cl100k_base)
- Secret redaction (AWS keys, private keys, GitHub tokens, JWTs, API keys)
- Budget enforcement (priority-based artifact selection)

### Implementation
- New crates: `ctx-tokens`, `ctx-security`, `ctx-engine`
- BLAKE3 hashing for reproducibility
- Regex-based redaction patterns
- ProcessedArtifact abstraction for render pipeline

---

## M1: Packs + Persistence (2025-12) ✅

**Summary**: Core pack management with SQLite storage and blob content-addressable storage.

### Features
- `ctx pack create <name> [--tokens N]`
- `ctx pack list`
- `ctx pack show <pack>`
- `ctx pack add <pack> <source> [--priority N]`
- `ctx pack remove <pack> <artifact-id>`

### Sources Supported
- `file:<path>` - Single files with optional line ranges
- `text:<content>` - Inline text snippets
- `glob:<pattern>` - Multiple files via glob patterns
- `md_dir:<path>` - Markdown directories (recursive)

### Implementation
- SQLite database with WAL mode
- Content-addressable blob storage (BLAKE3)
- Migration system with versioning
- Pack/artifact domain models
- Transaction-safe artifact creation

### Performance Improvements
- Single-query pack lookup (by name or ID)
- Migration check optimization (~50ms saved)
- Storage singleton pattern (~100ms saved per command)
- Row-to-struct helper extraction (eliminated 60 lines of duplication)

---

## Architecture

### Crates
- `ctx-cli`: CLI binary and command handlers
- `ctx-core`: Domain models (Pack, Artifact, RenderPolicy)
- `ctx-storage`: SQLite + blob storage with migrations
- `ctx-sources`: Source handlers (file, text, glob, collection)
- `ctx-security`: Redaction engine
- `ctx-tokens`: Token estimation (tiktoken)
- `ctx-engine`: Render orchestration
- `ctx-mcp`: MCP server

### Tech Stack
- Rust 1.75+
- SQLite 3.x with sqlx
- Tokio async runtime
- BLAKE3 hashing
- tiktoken-rs for token estimation
- Axum for MCP HTTP server

---

## Next: M4 Hardening

Planned features:
- Git diff source handler
- Configuration system (~/.ctx/config.toml)
- Enhanced security (denylist, validation)
- Integration tests
- Documentation improvements
