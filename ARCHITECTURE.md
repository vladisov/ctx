# Architecture

High-level overview of ctx's design and implementation.

---

## Design Principles

1. **Deterministic**: Same inputs always produce same outputs (content-addressable storage, stable sorting)
2. **Simple**: Straightforward code, minimal abstractions, clear separation of concerns
3. **Performant**: <200ms typical operations, connection pooling, compiled patterns
4. **Secure**: Redaction by default, denylist for sensitive files, preview before sending

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                            ctx CLI                                   │
│  Commands: @, create, add, rm, ls, show, preview, cp, delete,       │
│            suggest, lint, init, sync, save, mcp, ui, completions    │
└─────────────────────────────────────────────────────────────────────┘
                                  │
        ┌─────────────┬───────────┼───────────┬─────────────┐
        │             │           │           │             │
┌───────▼──────┐ ┌────▼────┐ ┌────▼────┐ ┌────▼────┐ ┌──────▼──────┐
│   Storage    │ │ Engine  │ │   MCP   │ │   TUI   │ │   Suggest   │
│  (SQLite +   │ │(Render  │ │ Server  │ │(Terminal│ │ (Smart      │
│    Blobs)    │ │pipeline)│ │(JSON-RPC│ │   UI)   │ │  context)   │
└───────┬──────┘ └────┬────┘ │+ REST)  │ └─────────┘ └─────────────┘
        │             │      └─────────┘
        │      ┌──────┴──────┬──────────┬──────────┐
        │      │             │          │          │
        │  ┌───▼────┐  ┌─────▼───┐ ┌────▼────┐ ┌───▼──────┐
        │  │Sources │  │ Tokens  │ │Security │ │  Config  │
        │  │(File,  │  │ (Token  │ │(Redact, │ │  (TOML)  │
        │  │Git,URL)│  │  Est.)  │ │ Deny)   │ └──────────┘
        │  └────────┘  └─────────┘ └─────────┘
        │
   ┌────▼─────┐
   │   Core   │
   │ (Domain) │
   └──────────┘
```

---

## Crates

### `ctx-cli` (Command Line Interface)
- **Purpose**: User-facing commands and entry point
- **Key files**: `main.rs`, `cli.rs`, `commands/*.rs`
- **Commands**: `@`, `create`, `add`, `rm`, `ls`, `show`, `preview`, `cp`, `delete`, `suggest`, `lint`, `init`, `sync`, `save`, `mcp`, `ui`, `completions`
- **Lines**: ~1,200

### `ctx-core` (Domain Models)
- **Purpose**: Core data structures and render engine
- **Key types**: `Pack`, `Artifact`, `RenderPolicy`, `ArtifactType`, `RenderEngine`
- **Responsibilities**: Domain logic, serialization, budget enforcement
- **Lines**: ~350

### `ctx-storage` (Persistence Layer)
- **Purpose**: SQLite database + blob storage
- **Key files**: `db.rs`, `blob.rs`, `models.rs`
- **Responsibilities**: CRUD operations, migrations, content-addressable storage
- **Features**: Connection pooling, WAL mode, transactions
- **Lines**: ~630

### `ctx-sources` (Source Handlers)
- **Purpose**: Parse and load different artifact types
- **Handlers**: `FileHandler`, `TextHandler`, `CollectionHandler`, `GitHandler`, `UrlHandler`
- **Key files**: `handler.rs`, `file.rs`, `text.rs`, `collection.rs`, `git.rs`, `url.rs`, `denylist.rs`
- **Responsibilities**: URI parsing, content loading, collection expansion, denylist validation
- **Lines**: ~550

### `ctx-security` (Redaction)
- **Purpose**: Secret detection and redaction
- **Patterns**: AWS keys, GitHub tokens, JWTs, API keys, private keys
- **Method**: Regex-based pattern matching
- **Lines**: ~120

### `ctx-tokens` (Token Estimation)
- **Purpose**: Estimate token counts
- **Method**: tiktoken-rs (cl100k_base encoding)
- **Lines**: ~65

### `ctx-engine` (Render Orchestration)
- **Purpose**: Coordinate rendering pipeline
- **Pipeline**: Load → Expand → Redact → Estimate → Sort → Budget → Concatenate → Hash
- **Key file**: `lib.rs` with `Renderer` struct
- **Lines**: ~390

### `ctx-config` (Configuration)
- **Purpose**: TOML configuration management
- **Files**: `~/.ctx/config.toml`, `ctx.toml` (project-local)
- **Features**: Auto-creation, defaults, validation, project namespacing
- **Lines**: ~330

### `ctx-mcp` (MCP Server)
- **Purpose**: JSON-RPC 2.0 server + REST API for AI agents
- **Protocol**: Model Context Protocol (2025-03-26)
- **Tools**: `ctx_packs_list`, `ctx_packs_get`, `ctx_packs_preview`, `ctx_packs_load`, `ctx_packs_create`, `ctx_packs_add_artifact`, `ctx_packs_delete`
- **Transports**: HTTP (port 17373), stdio (for Claude Code)
- **REST API**: `/api/packs`, `/api/suggest` (for VS Code, ChatGPT Actions)
- **Lines**: ~700

### `ctx-tui` (Terminal UI)
- **Purpose**: Interactive terminal interface
- **Framework**: ratatui
- **Features**: Pack browser, artifact preview, keyboard navigation
- **Lines**: ~1,270

### `ctx-suggest` (Smart Context)
- **Purpose**: Intelligent file suggestions
- **Signals**: Git co-change history, import graph analysis
- **Parsers**: Rust, TypeScript/JavaScript, Python
- **Lines**: ~700

---

## Artifact Types

| Type | URI Scheme | Description |
|------|------------|-------------|
| `File` | `file:path` | Single file |
| `FileRange` | `file:path --start N --end M` | File line range |
| `Markdown` | `md:path` | Markdown file |
| `Text` | `text:content` | Inline text |
| `CollectionMdDir` | `md_dir:path` | Directory of markdown files |
| `CollectionGlob` | `glob:pattern` | Files matching glob pattern |
| `GitDiff` | `git:diff --base=main` | Git diff output |
| `Url` | `url:https://...` | Web page (HTML→text) |

---

## MCP Server

### Setup Options

**Stdio (recommended for Claude Code)**:
```bash
claude mcp add ctx -- ctx mcp --stdio
```

**HTTP**:
```bash
ctx mcp --port 17373
claude mcp add --transport http ctx http://127.0.0.1:17373
```

### Available Tools

| Tool | Description |
|------|-------------|
| `ctx_packs_list` | List all packs |
| `ctx_packs_get` | Get pack metadata and artifacts |
| `ctx_packs_preview` | Preview with token counts |
| `ctx_packs_load` | **Load pack content** (for LLM context) |
| `ctx_packs_create` | Create new pack |
| `ctx_packs_add_artifact` | Add artifact to pack |
| `ctx_packs_delete` | Delete pack |

### REST API (for VS Code, ChatGPT)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/packs` | GET | List packs |
| `/api/packs` | POST | Create pack |
| `/api/packs/:name` | GET | Get pack |
| `/api/packs/:name` | DELETE | Delete pack |
| `/api/packs/:name/render` | GET | Render pack content |
| `/api/packs/:name/artifacts` | GET | List artifacts |
| `/api/packs/:name/artifacts` | POST | Add artifact |
| `/api/suggest` | GET | Get file suggestions |

---

## Smart Context Selection

### Signals

**Git Co-Change** (weight: 0.5)
- Analyzes last 500 commits
- Identifies files frequently changed together
- Score = co-change count / max count

**Import Graph** (weight: 0.5)
- Parses imports with regex per language
- Builds bidirectional graph (imports + imported_by)
- Scores: direct import (0.8), imported_by (0.9), transitive (0.3)

### Supported Languages

| Language | Patterns |
|----------|----------|
| Rust | `use crate::`, `mod foo;` |
| TypeScript/JS | `import from`, `require()` |
| Python | `import`, `from X import` |

---

## Data Flow

### Quick Command (`ctx @`)
```
User: ctx @ src/auth.rs
  │
  ├─> Read file content
  ├─> SuggestionEngine.suggest(file)
  │     ├─> Git co-change signal
  │     └─> Import graph signal
  ├─> Score and rank related files
  ├─> Combine file + related files
  ├─> Apply redaction
  └─> Copy to clipboard (or stdout)
```

### Pack Rendering
```
User: ctx preview demo
  │
  ├─> Storage.get_pack("demo")
  ├─> Storage.get_pack_artifacts(pack_id)
  │     └─> SELECT with priority DESC, added_at ASC
  ├─> Renderer.render_pack(pack_id)
  │     ├─> For each artifact:
  │     │     ├─> Expand collections (glob, md_dir)
  │     │     ├─> Load content (from handler or cache)
  │     │     ├─> Redact secrets (regex patterns)
  │     │     └─> Estimate tokens (tiktoken)
  │     ├─> Sort artifacts (deterministic)
  │     ├─> Apply budget (priority-based)
  │     ├─> Concatenate payload
  │     └─> Compute BLAKE3 hash
  └─> Display RenderResult
```

---

## Storage Schema

### Tables

**packs**
```sql
pack_id TEXT PRIMARY KEY
name TEXT UNIQUE
policies_json TEXT  -- RenderPolicy as JSON
created_at INTEGER
updated_at INTEGER
```

**artifacts**
```sql
artifact_id TEXT PRIMARY KEY
type_json TEXT      -- ArtifactType as JSON
source_uri TEXT
content_hash TEXT   -- BLAKE3 hash
meta_json TEXT      -- ArtifactMetadata as JSON
token_est INTEGER
created_at INTEGER
```

**pack_items** (many-to-many)
```sql
pack_id TEXT
artifact_id TEXT
priority INTEGER DEFAULT 0
added_at INTEGER
PRIMARY KEY (pack_id, artifact_id)
INDEX ON (pack_id, priority DESC, added_at ASC)
```

### Blob Storage

Content-addressable file system:
```
~/.local/share/com.ctx.ctx/blobs/
  ├── a3/
  │   └── a3f2b1c4d5e6f7... (file content)
  └── b4/
      └── b4a8c9d0e1f2... (file content)
```

---

## Project-Local Packs

**File**: `ctx.toml` in project root

```toml
[config]
default_budget = 50000

[packs.style-guide]
budget = 25000
artifacts = [
    { source = "file:CONTRIBUTING.md", priority = 10 },
    { source = "text:Use async/await patterns", priority = 100 },
]
```

**Commands**:
- `ctx init` - Create ctx.toml
- `ctx sync` - Import packs from ctx.toml
- `ctx save <pack>` - Export pack to ctx.toml

Packs are namespaced: `project-name:pack-name`

---

## Security Model

### 1. Redaction (Always On)

**Patterns**:
- AWS keys: `AKIA[0-9A-Z]{16}`
- GitHub tokens: `gh[ps]_[a-zA-Z0-9]{36,}`
- JWTs: `eyJ...`
- Private keys: `-----BEGIN.*PRIVATE KEY-----`
- Generic API keys: `api_key.*[a-zA-Z0-9]{20,}`

### 2. Denylist (Default Patterns)

**Blocked**:
- `**/.env*` - Environment variables
- `**/.aws/**` - AWS credentials
- `**/secrets/**` - Secret directories
- `**/*.key`, `**/*.pem` - Key files

### 3. Preview (User Control)

Users must explicitly use `--payload` to see content.

---

## Configuration

**Global**: `~/.ctx/config.toml`

```toml
budget_tokens = 128000

[denylist]
patterns = ["**/.env*", "**/*.key"]

[mcp]
host = "127.0.0.1"
port = 17373
read_only = false
```

---

## Testing

### Test Coverage
- **Unit tests**: 58 tests across all crates
- **Integration tests**: 16 end-to-end CLI tests
- **Run**: `cargo test --all && ./tests/integration_test.sh`

### Test Commands
```bash
cargo test --all           # Unit tests
cargo clippy --all-targets # Lints
cargo fmt --check          # Formatting
./tests/integration_test.sh # Integration
```

---

## Dependencies

**Core**:
- `tokio` - Async runtime
- `sqlx` - Database
- `blake3` - Hashing
- `serde` / `serde_json` - Serialization

**CLI**:
- `clap` - Argument parsing
- `clap_complete` - Shell completions
- `arboard` - Clipboard

**UI**:
- `ratatui` - Terminal UI
- `axum` - HTTP server

**Specialized**:
- `tiktoken-rs` - Token estimation
- `regex` - Pattern matching
- `reqwest` - HTTP client (URL artifacts)
- `ignore` - Gitignore-aware walking

---

## Metrics

| Metric | Value |
|--------|-------|
| Lines of code | ~8,300 |
| Crates | 10 |
| CLI commands | 17 |
| Source handlers | 5 |
| Artifact types | 8 |
| MCP tools | 7 |
| Performance | <200ms typical |
| Binary size | ~6MB (stripped) |
| Startup time | ~20ms |

---

## Build & Deploy

```bash
# Development
cargo build
cargo test --all
./tests/integration_test.sh

# Release
cargo build --release
# Binary: target/release/ctx

# Install
cargo install --path crates/ctx-cli

# Shell completions
ctx completions zsh > ~/.zfunc/_ctx
ctx completions bash >> ~/.bash_completion
ctx completions fish > ~/.config/fish/completions/ctx.fish
```
