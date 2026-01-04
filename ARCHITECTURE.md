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
┌─────────────────────────────────────────────────────────────┐
│                         ctx CLI                              │
│  (User commands: pack, mcp)                                 │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────▼──────┐    ┌────────▼────────┐    ┌──────▼──────┐
│   Storage    │    │     Engine      │    │     MCP     │
│  (SQLite +   │◄───│  (Orchestrate   │◄───│   Server    │
│    Blobs)    │    │    rendering)   │    │ (JSON-RPC)  │
└───────┬──────┘    └────────┬────────┘    └─────────────┘
        │                    │
        │           ┌────────┼────────┐
        │           │        │        │
        │     ┌─────▼──┐ ┌──▼────┐ ┌─▼────────┐
        │     │Sources │ │Tokens │ │Security  │
        │     │(File,  │ │(Token │ │(Redact,  │
        │     │ Git)   │ │ Est.) │ │ Deny)    │
        │     └────────┘ └───────┘ └──────────┘
        │
   ┌────▼─────┐
   │  Config  │
   │ (TOML)   │
   └──────────┘
```

---

## Crates

### `ctx-cli` (Command Line Interface)
- **Purpose**: User-facing commands
- **Key files**: `main.rs`, `cli.rs`, `commands/pack.rs`, `commands/mcp.rs`
- **Responsibilities**: Parse args, load config, call storage/engine
- **Lines**: ~350

### `ctx-core` (Domain Models)
- **Purpose**: Core data structures
- **Key types**: `Pack`, `Artifact`, `Snapshot`, `RenderPolicy`, `ArtifactType`
- **Responsibilities**: Domain logic, serialization
- **Lines**: ~250

### `ctx-storage` (Persistence Layer)
- **Purpose**: SQLite database + blob storage
- **Key files**: `db.rs`, `blob.rs`, `migrations/001_initial.sql`
- **Responsibilities**: CRUD operations, migrations, content-addressable storage
- **Features**: Connection pooling, WAL mode, transactions
- **Lines**: ~450

### `ctx-sources` (Source Handlers)
- **Purpose**: Parse and load different artifact types
- **Handlers**: `FileHandler`, `TextHandler`, `CollectionHandler`, `GitHandler`
- **Key files**: `handler.rs`, `file.rs`, `text.rs`, `collection.rs`, `git.rs`, `denylist.rs`
- **Responsibilities**: URI parsing, content loading, collection expansion, denylist validation
- **Lines**: ~450

### `ctx-security` (Redaction)
- **Purpose**: Secret detection and redaction
- **Patterns**: AWS keys, GitHub tokens, JWTs, API keys, private keys
- **Method**: Regex-based pattern matching
- **Lines**: ~118

### `ctx-tokens` (Token Estimation)
- **Purpose**: Estimate token counts
- **Method**: tiktoken-rs (cl100k_base encoding)
- **Lines**: ~61

### `ctx-engine` (Render Orchestration)
- **Purpose**: Coordinate rendering pipeline
- **Pipeline**: Load → Expand → Redact → Estimate → Sort → Budget → Concatenate → Hash
- **Key file**: `lib.rs` with `Renderer` struct
- **Lines**: ~147

### `ctx-config` (Configuration)
- **Purpose**: TOML configuration management
- **File**: `~/.ctx/config.toml`
- **Features**: Auto-creation, defaults, validation
- **Lines**: ~143

### `ctx-mcp` (MCP Server)
- **Purpose**: JSON-RPC 2.0 server for AI agents
- **Protocol**: Model Context Protocol
- **Tools**: `ctx_packs_list`, `ctx_packs_get`, `ctx_packs_preview`, `ctx_packs_snapshot`
- **Server**: Axum on port 17373
- **Lines**: ~200

---

## Data Flow

### Pack Creation
```
User: ctx pack create demo --tokens 5000
  │
  ├─> CLI parses args
  ├─> Load config (defaults)
  ├─> Create Pack object (with RenderPolicy)
  └─> Storage.create_pack() → SQLite INSERT
```

### Adding Artifacts
```
User: ctx pack add demo file:README.md
  │
  ├─> CLI parses source URI
  ├─> SourceHandlerRegistry.parse("file:README.md")
  │     └─> FileHandler.parse() → Artifact
  ├─> Check denylist (glob patterns)
  ├─> SourceHandlerRegistry.load(artifact)
  │     └─> FileHandler.load() → content (string)
  ├─> Storage.add_artifact_to_pack_with_content()
  │     ├─> BlobStore.store(content) → BLAKE3 hash
  │     ├─> BEGIN TRANSACTION
  │     ├─> INSERT INTO artifacts
  │     ├─> INSERT INTO pack_items
  │     └─> COMMIT
  └─> Success
```

### Preview (Rendering)
```
User: ctx pack preview demo
  │
  ├─> Storage.get_pack("demo")
  ├─> Storage.get_pack_artifacts(pack_id)
  │     └─> SELECT with priority DESC, added_at ASC
  ├─> Renderer.render_pack(pack_id)
  │     ├─> For each artifact:
  │     │     ├─> Expand collections (md_dir, glob)
  │     │     ├─> Load content (from blob or handler)
  │     │     ├─> Redact secrets (regex patterns)
  │     │     └─> Estimate tokens (tiktoken)
  │     ├─> Sort artifacts (deterministic)
  │     ├─> Apply budget (priority-based selection)
  │     ├─> Concatenate payload
  │     └─> Compute BLAKE3 hash
  └─> Display RenderResult
```

### Snapshot
```
User: ctx pack snapshot demo --label "v1.0"
  │
  ├─> Renderer.render_pack(pack_id)
  ├─> Create Snapshot object
  │     ├─> render_hash (from render result)
  │     ├─> payload_hash (BLAKE3 of payload)
  │     └─> label
  └─> Storage.create_snapshot() → SQLite INSERT
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
FOREIGN KEY (pack_id) REFERENCES packs ON DELETE CASCADE
FOREIGN KEY (artifact_id) REFERENCES artifacts ON DELETE CASCADE
INDEX ON (pack_id, priority DESC, added_at ASC)  -- Query optimization
```

**snapshots**
```sql
snapshot_id TEXT PRIMARY KEY
label TEXT
render_hash TEXT
payload_hash TEXT
created_at INTEGER
INDEX ON (render_hash)
INDEX ON (created_at DESC)
```

### Blob Storage

Content-addressable file system:
```
~/.local/share/ctx/ctx/blobs/blake3/
  ├── a3/
  │   └── a3f2b1c4d5e6f7... (file content)
  ├── b4/
  │   └── b4a8c9d0e1f2... (file content)
  ...
```

**Benefits**:
- Automatic deduplication
- Integrity verification (hash = filename)
- Immutable storage

---

## Key Algorithms

### Deterministic Rendering

**Requirement**: Same pack → same render_hash (reproducibility)

**Implementation**:
1. Sort artifacts by ID (stable order)
2. Process in order (no parallel randomness)
3. Use content hashes in render_hash computation
4. BLAKE3 for cryptographic hashing

**Verification**:
```rust
let result1 = render_pack(pack_id).await?;
let result2 = render_pack(pack_id).await?;
assert_eq!(result1.render_hash, result2.render_hash);
```

### Budget Enforcement

**Algorithm**:
```rust
fn apply_budget(artifacts: Vec<Artifact>, budget: usize) -> (Vec, Vec) {
    let mut included = vec![];
    let mut excluded = vec![];
    let mut total = 0;

    for artifact in artifacts {  // Already sorted by priority DESC
        if total + artifact.tokens <= budget {
            total += artifact.tokens;
            included.push(artifact);
        } else {
            excluded.push((artifact, "over_budget"));
        }
    }

    (included, excluded)
}
```

**Priority**: Higher priority artifacts are processed first (sorted in SQL query)

---

## Performance Optimizations

1. **Storage pool**: SqlitePool created once, reused across commands (~100ms saved)
2. **Migration check**: Tracked in `_migrations` table, runs once (~50ms saved)
3. **Compiled patterns**: Denylist and redaction patterns compiled at startup
4. **Single query pack lookup**: `WHERE pack_id = ? OR name = ?` (50% faster)
5. **Indexed queries**: pack_items has index on (pack_id, priority DESC, added_at ASC)
6. **WAL mode**: SQLite Write-Ahead Logging for better concurrency

**Result**: Typical operations <200ms

---

## Security Model

### 1. Redaction (Always On)

**Patterns**:
- AWS keys: `AKIA[0-9A-Z]{16}`
- GitHub tokens: `gh[ps]_[a-zA-Z0-9]{36,}`
- JWTs: `eyJ...eyJ...`
- Private keys: `-----BEGIN.*PRIVATE KEY-----`
- API keys: `api_key.*[a-zA-Z0-9]{20,}`

**Process**: Regex replacement with `[REDACTED:TYPE]`

### 2. Denylist (Default Patterns)

**Blocked**:
- `**/.env*` - Environment variables
- `**/.aws/**` - AWS credentials
- `**/secrets/**` - Secret directories
- `**/*.key`, `**/*.pem` - Key files
- `**/*_rsa` - SSH keys

**Validation**: On `pack add`, before storage

### 3. Preview (User Control)

Users must explicitly `--show-payload` to see content. Default shows summary only.

---

## Configuration

**File**: `~/.ctx/config.toml`

**Sections**:
```toml
budget_tokens = 128000         # Default pack budget

[denylist]
patterns = ["**/.env*", ...]   # Glob patterns

[mcp]
host = "127.0.0.1"             # MCP server bind address
port = 17373                   # MCP server port
read_only = false              # Read-only mode
```

**Loading**: On CLI startup, cached for session

---

## Testing Strategy

### Unit Tests
- Each crate has `#[cfg(test)] mod tests`
- Test coverage: Core logic, edge cases, error handling
- Run: `cargo test`

### Integration Tests
- Shell script: `tests/integration_test.sh`
- Tests: End-to-end CLI workflows
- Coverage: Pack CRUD, rendering, denylist, determinism
- Run: `./tests/integration_test.sh`

### Test Artifacts
```rust
fn create_test_artifact(id: &str, content: &str, tokens: usize) -> ProcessedArtifact {
    ProcessedArtifact {
        artifact: Artifact::new(ArtifactType::Text { content: content.to_string() }, ...),
        content: content.to_string(),
        token_count: tokens,
        redacted: false,
    }
}
```

---

## Future Considerations

**Not in MVP, but architecturally sound for**:
- Remote storage (Storage trait abstraction ready)
- Plugin system (SourceHandler trait extensible)
- Web UI (MCP server provides API layer)
- Parallel rendering (artifacts are independent)
- Caching (content-addressable storage enables it)

**Current design doesn't block** any of these enhancements.

---

## Dependencies

**Core**:
- `tokio` - Async runtime
- `sqlx` - Database (compile-time checked queries)
- `blake3` - Fast cryptographic hashing
- `serde` / `serde_json` - Serialization

**CLI**:
- `clap` - Argument parsing

**Specialized**:
- `tiktoken-rs` - Token estimation
- `regex` - Pattern matching (redaction, denylist)
- `glob` - Path patterns
- `axum` - HTTP server (MCP)
- `toml` - Configuration

**Total**: ~15 direct dependencies (minimal)

---

## Metrics

- **Lines of code**: ~2,800 (Rust)
- **Crates**: 9
- **Commands**: 8 (create, list, show, add, remove, preview, snapshot, mcp)
- **Source handlers**: 4 (file, text, collection, git)
- **Artifact types**: 7 (File, FileRange, Text, Markdown, CollectionMdDir, CollectionGlob, GitDiff)
- **Performance**: <200ms typical operation
- **Test coverage**: 10 integration + unit tests per crate

---

## Build & Deploy

```bash
# Development
cargo build
cargo test
./tests/integration_test.sh

# Release
cargo build --release
# Binary: target/release/ctx

# Install
cargo install --path crates/ctx-cli
```

**Binary size**: ~5MB (stripped release build)
**Startup time**: ~20ms
**Memory usage**: ~15MB typical
