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
│                            ctx-cli                                   │
│  Commands: @, create, add, rm, ls, show, preview, cp, delete,       │
│            suggest, lint, init, sync, save, mcp, ui, completions    │
│  Config: Global (~/.ctx/config.toml) + Project (ctx.toml)           │
└─────────────────────────────────────────────────────────────────────┘
                                  │
        ┌─────────────┬───────────┼───────────┬─────────────┐
        │             │           │           │             │
┌───────▼──────┐ ┌────▼────┐ ┌────▼────┐ ┌────▼────┐ ┌──────▼──────┐
│  ctx-storage │ │ctx-engine│ │ ctx-mcp │ │ ctx-tui │ │ ctx-suggest │
│  SQLite +    │ │ Render   │ │JSON-RPC │ │Terminal │ │   Smart     │
│  Blob store  │ │ pipeline │ │+ REST   │ │   UI    │ │  context    │
└───────┬──────┘ └────┬────┘ └─────────┘ └─────────┘ └─────────────┘
        │             │
        │      ┌──────┴──────┐
        │      │             │
        │  ┌───▼────┐   ┌────▼─────┐
        │  │ctx-    │   │ ctx-core │
        │  │sources │   │ Domain + │
        │  │File,Git│   │ Tokens + │
        │  │URL,Deny│   │ Security │
        │  └────────┘   └──────────┘
        │                    │
        └────────────────────┘
```

---

## Crates (7 total)

### `ctx-core` — Domain Models & Utilities
**Purpose**: Foundation layer with core types, token estimation, and security redaction.

**Core Types** (`artifact.rs`, `pack.rs`):
- `Artifact` — Content unit with type, source URI, content hash, metadata
- `ArtifactType` — Enum: File, FileRange, Text, Markdown, CollectionMdDir, CollectionGlob, GitDiff, Url
- `Pack` — Named collection of artifacts with render policies
- `RenderPolicy` — Budget tokens, ordering strategy

**Render Engine** (`render.rs`):
- `RenderEngine` — Deterministic payload generation
- `ProcessedArtifact` — Artifact with loaded content and token count
- `RenderResult` — Final output with payload, hash, included/excluded lists

**Token Estimation** (`tokens.rs`):
- `TokenEstimator` — tiktoken-rs wrapper (cl100k_base encoding for GPT-4)
- Methods: `estimate(text)`, `estimate_batch(texts)`

**Security** (`security.rs`):
- `Redactor` — Regex-based secret detection and replacement
- Patterns: AWS keys, GitHub tokens, JWTs, API keys, private keys, bearer tokens
- Output: `[REDACTED:TYPE]` placeholders + `RedactionInfo` tracking

**Lines**: ~390

---

### `ctx-cli` — Command Line Interface
**Purpose**: User-facing entry point with all commands and configuration management.

**Commands** (`commands/*.rs`):
| Command | Description |
|---------|-------------|
| `@` (quick) | File + related files → clipboard |
| `create` | Create new pack |
| `add` | Add artifact to pack |
| `rm` | Remove artifact |
| `ls` | List all packs |
| `show` | Show pack details |
| `preview` | Preview rendered pack |
| `cp` | Copy pack to clipboard |
| `delete` | Delete pack |
| `lint` | Check missing dependencies |
| `suggest` | Get related file suggestions |
| `init` | Create ctx.toml |
| `sync` | Import packs from ctx.toml |
| `save` | Export packs to ctx.toml |
| `mcp` | Start MCP server |
| `ui` | Launch TUI or web UI |
| `completions` | Generate shell completions |

**Configuration** (`config.rs`):
- `Config` — Global settings (~/.ctx/config.toml): budget, denylist patterns, MCP settings
- `ProjectConfig` — Project-local (ctx.toml): pack definitions, default budget
- `PackDefinition` — Pack schema for ctx.toml with artifacts and priorities

**Lines**: ~1,580

---

### `ctx-storage` — Persistence Layer
**Purpose**: SQLite database for metadata + content-addressable blob storage.

**Database** (`db.rs`):
- `Storage` — Main interface with connection pool (sqlx)
- Tables: `packs`, `artifacts`, `pack_items` (many-to-many)
- Features: WAL mode, migrations, transactions, upserts
- Queries: Priority-ordered artifact retrieval, pack lookup by name/ID

**Blob Storage** (`blob.rs`):
- Content-addressable file system at `~/.local/share/com.ctx.ctx/blobs/`
- Structure: `ab/abcdef123...` (2-char prefix directories)
- BLAKE3 hashing for deduplication
- Read/write with automatic directory creation

**Lines**: ~710

---

### `ctx-sources` — Source Handlers
**Purpose**: Parse URI schemes and load artifact content from various sources.

**Handler Registry** (`handler.rs`):
- `SourceHandlerRegistry` — Routes URIs to appropriate handlers
- `SourceOptions` — Range, max_files, exclude patterns, priority

**Handlers**:
| Handler | URI Scheme | Functionality |
|---------|------------|---------------|
| `FileHandler` | `file:path` | Read files, support line ranges |
| `TextHandler` | `text:content` | Inline text content |
| `CollectionHandler` | `glob:`, `md_dir:` | Expand patterns to file lists |
| `GitHandler` | `git:diff` | Run git commands, parse diff output |
| `UrlHandler` | `url:https://` | Fetch web pages, convert HTML→text |

**Denylist** (`denylist.rs`):
- `Denylist` — Glob pattern matching for sensitive files
- Default patterns: `.env*`, `.aws/**`, `secrets/**`, `*.key`, `*.pem`
- Methods: `is_denied(path)`, `matching_pattern(path)`

**Lines**: ~750

---

### `ctx-engine` — Render Orchestration
**Purpose**: Coordinate the full rendering pipeline from pack to final payload.

**Renderer** (`lib.rs`):
- `Renderer` — Orchestrates: Storage → Sources → Redactor → TokenEstimator → RenderEngine
- `render_pack(pack_id)` — Full pipeline for single pack
- `render_request(pack_ids)` — Multi-pack rendering with merged output

**Pipeline Steps**:
1. Load pack and artifacts from storage
2. Expand collections (glob patterns, md_dir)
3. Load content via source handlers (disk or cached blob)
4. Redact secrets with regex patterns
5. Estimate token counts
6. Sort by priority (deterministic)
7. Apply budget (include until limit)
8. Concatenate payload with headers
9. Compute BLAKE3 hash for reproducibility

**Lines**: ~390

---

### `ctx-mcp` — MCP Server
**Purpose**: Model Context Protocol server for AI agent integration.

**Protocol** (`protocol.rs`, `stdio.rs`):
- JSON-RPC 2.0 over HTTP or stdio
- MCP spec version: 2025-03-26
- Capabilities: tools (list, call)

**Tools** (`tools.rs`):
| Tool | Parameters | Returns |
|------|------------|---------|
| `ctx_packs_list` | — | Pack names and IDs |
| `ctx_packs_get` | name | Pack metadata + artifacts |
| `ctx_packs_preview` | name | Token counts, included/excluded |
| `ctx_packs_load` | name | **Full rendered content** |
| `ctx_packs_create` | name, budget? | Created pack info |
| `ctx_packs_add_artifact` | pack, source, priority? | Added artifact info |
| `ctx_packs_delete` | name | Confirmation |

**REST API** (`server.rs`):
- `/api/packs` — CRUD for packs
- `/api/packs/:name/render` — Get rendered content
- `/api/packs/:name/artifacts` — Manage artifacts
- `/api/suggest` — File suggestions

**Lines**: ~800

---

### `ctx-tui` — Terminal UI
**Purpose**: Interactive terminal interface for pack management.

**Components**:
- `App` (`app.rs`) — Application state, event handling
- `UI` (`ui.rs`) — Ratatui rendering, layouts
- `FileBrowser` (`file_browser.rs`) — Directory navigation for adding files

**Features**:
- Pack list with creation/deletion
- Artifact browser with priority display
- File browser for adding new artifacts
- Preview pane with token counts
- Keyboard navigation (vim-style)

**Lines**: ~1,500

---

### `ctx-suggest` — Smart Context Selection
**Purpose**: Intelligent file suggestions based on code relationships.

**Suggestion Engine** (`lib.rs`):
- `SuggestionEngine` — Combines signals, ranks results
- `Suggestion` — Path, score (0-1), reasons
- `SuggestConfig` — Weights, thresholds, limits

**Signals** (`signals/`):
| Signal | Weight | Method |
|--------|--------|--------|
| Git Co-Change | 0.5 | Analyze 500 commits, count co-occurrences |
| Import Graph | 0.5 | Parse imports, build bidirectional graph |

**Import Parsers** (`parsers/`):
| Language | Patterns Detected |
|----------|-------------------|
| Rust | `use crate::`, `mod foo;`, `use super::` |
| TypeScript/JS | `import from`, `require()`, `export from` |
| Python | `import x`, `from x import y` |

**Scoring**:
- Direct import: 0.8
- Imported by: 0.9
- Transitive (1-hop): 0.3
- Git co-change: count / max_count

**Lines**: ~700

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
- **Unit tests**: 51 tests across all crates
- **Integration tests**: 16 end-to-end CLI tests
- **Run**: `cargo test --workspace && ./tests/integration_test.sh`

### Test Commands
```bash
cargo test --workspace     # Unit tests
cargo clippy --workspace   # Lints
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
| Crates | 7 |
| CLI commands | 17 |
| Source handlers | 5 |
| Artifact types | 8 |
| MCP tools | 7 |
| Unit tests | 51 |
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
