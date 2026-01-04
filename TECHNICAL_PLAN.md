# ctx — Technical Plan (Rust Implementation)

**Version**: MVP v1.0
**Language**: Rust
**Target**: CLI + MCP Server for repeatable LLM context management
**Timeline**: 8 weeks (1 senior Rust engineer)

---

## Table of Contents

1. [Technology Stack (Rust)](#1-technology-stack-rust)
2. [Project Structure](#2-project-structure)
3. [Architecture & Design](#3-architecture--design)
4. [Core Implementation Details](#4-core-implementation-details)
5. [MCP Server Implementation](#5-mcp-server-implementation)
6. [Milestone Implementation Plan](#6-milestone-implementation-plan)
7. [Testing Strategy](#7-testing-strategy)
8. [Build & Distribution](#8-build--distribution)
9. [Key Risks & Mitigations](#9-key-risks--mitigations)
10. [Development Environment Setup](#10-development-environment-setup)

---

## 1. Technology Stack (Rust)

### 1.1 Core Dependencies

```toml
[dependencies]
# CLI framework
clap = { version = "4.4", features = ["derive", "cargo", "env"] }
clap_complete = "4.4"

# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "sqlite"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Hashing & crypto
sha2 = "0.10"
blake3 = "1.5"  # Faster alternative to SHA256

# Token estimation
tiktoken-rs = "0.5"

# File system & glob
glob = "0.3"
walkdir = "2.4"
ignore = "0.4"  # Respects .gitignore

# Git operations
git2 = "0.18"

# HTTP server (for MCP)
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Regex
regex = "1.10"
lazy_static = "1.4"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Config management
config = "0.13"
directories = "5.0"  # Cross-platform config paths

# Command execution
which = "5.0"

[dev-dependencies]
tempfile = "3.8"
assert_cmd = "2.0"
predicates = "3.0"
mockall = "0.12"
proptest = "1.4"
criterion = "0.5"
```

### 1.2 Rationale for Rust

**Advantages over Go**:
- **Type safety**: Stronger guarantees for reproducibility
- **Zero-cost abstractions**: Better performance for token estimation
- **Memory safety**: No GC pauses during rendering
- **Better error handling**: Result<T, E> enforces error checking
- **Strong ecosystem**: tokio, sqlx, clap are production-ready

**Trade-offs**:
- Slower initial development (stricter compiler)
- More complex async code (but better performance)
- Steeper learning curve for contributors

---

## 2. Project Structure

```
ctx/
├── Cargo.toml                 # Workspace root
├── Cargo.lock
├── .cargo/
│   └── config.toml           # Build config
├── crates/
│   ├── ctx-cli/              # Binary crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/     # CLI command handlers
│   │       │   ├── mod.rs
│   │       │   ├── pack.rs
│   │       │   └── mcp.rs
│   │       └── cli.rs        # Clap definitions
│   │
│   ├── ctx-core/             # Core library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pack.rs       # Pack domain model
│   │       ├── artifact.rs   # Artifact domain model
│   │       ├── snapshot.rs   # Snapshot domain model
│   │       ├── render.rs     # Render engine
│   │       ├── policy.rs     # Render policy
│   │       └── error.rs      # Custom error types
│   │
│   ├── ctx-storage/          # Persistence layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── db.rs         # SQLx database ops
│   │       ├── blob.rs       # Blob store
│   │       ├── migrations/   # SQL migrations
│   │       │   └── 001_initial.sql
│   │       └── models.rs     # DB models
│   │
│   ├── ctx-sources/          # Source handlers
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── handler.rs    # SourceHandler trait
│   │       ├── file.rs
│   │       ├── collection.rs
│   │       ├── git.rs
│   │       ├── command.rs
│   │       └── text.rs
│   │
│   ├── ctx-security/         # Security features
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── redactor.rs   # Secret redaction
│   │       └── denylist.rs   # Path denylist
│   │
│   ├── ctx-tokens/           # Token estimation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── estimator.rs
│   │
│   └── ctx-mcp/              # MCP server
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── server.rs     # Axum HTTP server
│           ├── protocol.rs   # JSON-RPC 2.0
│           ├── tools.rs      # MCP tools
│           └── schemas.rs    # Tool schemas
│
├── tests/                    # Integration tests
│   ├── integration/
│   │   ├── test_pack_operations.rs
│   │   ├── test_rendering.rs
│   │   └── test_mcp_server.rs
│   └── fixtures/
│       └── sample-repo/
│
├── benches/                  # Benchmarks
│   └── rendering.rs
│
├── docs/
│   ├── README.md
│   ├── ARCHITECTURE.md
│   ├── MCP_INTEGRATION.md
│   └── examples/
│       ├── style-pack.md
│       └── repo-pack.md
│
└── scripts/
    ├── install.sh
    └── test-e2e.sh
```

### 2.1 Cargo Workspace Configuration

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/ctx-cli",
    "crates/ctx-core",
    "crates/ctx-storage",
    "crates/ctx-sources",
    "crates/ctx-security",
    "crates/ctx-tokens",
    "crates/ctx-mcp",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["ctx contributors"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/ctx"

[workspace.dependencies]
# Shared dependencies across crates
ctx-core = { path = "crates/ctx-core" }
ctx-storage = { path = "crates/ctx-storage" }
ctx-sources = { path = "crates/ctx-sources" }
ctx-security = { path = "crates/ctx-security" }
ctx-tokens = { path = "crates/ctx-tokens" }
ctx-mcp = { path = "crates/ctx-mcp" }

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true  # Reduce binary size
```

---

## 3. Architecture & Design

### 3.1 Core Domain Models

```rust
// crates/ctx-core/src/pack.rs
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub policies: RenderPolicy,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPolicy {
    pub budget_tokens: usize,
    pub ordering: OrderingStrategy,
    pub redaction: RedactionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderingStrategy {
    PriorityThenTime,  // Default: priority DESC, added_at ASC
}

// crates/ctx-core/src/artifact.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub artifact_type: ArtifactType,
    pub source_uri: String,
    pub content_hash: String,
    pub metadata: ArtifactMetadata,
    pub token_estimate: usize,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArtifactType {
    File { path: String },
    FileRange { path: String, start: usize, end: usize },
    Markdown { path: String },
    CollectionMdDir {
        path: String,
        max_files: Option<usize>,
        exclude: Vec<String>,
        recursive: bool,
    },
    CollectionGlob { pattern: String },
    GitDiff { base: String, head: String },
    CommandOutput { command: String, cwd: String },
    Text { content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub size_bytes: usize,
    pub mime_type: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

// crates/ctx-core/src/snapshot.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub label: Option<String>,
    pub render_hash: String,
    pub payload_hash: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotItem {
    pub snapshot_id: String,
    pub artifact_id: String,
    pub content_hash: String,
    pub render_metadata: RenderItemMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderItemMetadata {
    pub included: bool,
    pub token_estimate: usize,
    pub exclusion_reason: Option<String>,
    pub redactions: Vec<RedactionInfo>,
}
```

### 3.2 Database Schema (SQLite)

```sql
-- crates/ctx-storage/src/migrations/001_initial.sql

-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA strict = ON;

CREATE TABLE packs (
    pack_id TEXT PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    policies_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE artifacts (
    artifact_id TEXT PRIMARY KEY NOT NULL,
    type TEXT NOT NULL,
    source_uri TEXT NOT NULL,
    content_hash TEXT,
    meta_json TEXT NOT NULL,
    token_est INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE pack_items (
    pack_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (pack_id, artifact_id),
    FOREIGN KEY (pack_id) REFERENCES packs(pack_id) ON DELETE CASCADE,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(artifact_id) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_pack_items_pack_ordering
    ON pack_items(pack_id, priority DESC, added_at ASC);

CREATE TABLE snapshots (
    snapshot_id TEXT PRIMARY KEY NOT NULL,
    label TEXT,
    render_hash TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_snapshots_render_hash ON snapshots(render_hash);
CREATE INDEX idx_snapshots_created ON snapshots(created_at DESC);

CREATE TABLE snapshot_items (
    snapshot_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    render_meta_json TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, artifact_id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id) ON DELETE CASCADE
) STRICT;
```

### 3.3 Blob Storage Strategy

```
~/.ctx/blobs/<algorithm>/<prefix>/<hash>

Examples:
~/.ctx/blobs/blake3/a3/a3f2b8c9d1e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0
~/.ctx/blobs/blake3/7e/7e4d5c1ab2f3e4d5c6b7a8f9e0d1c2b3a4f5e6d7c8b9a0f1e2d3c4b5a6f7e8

Benefits:
- Content-addressable (automatic deduplication)
- BLAKE3 is faster than SHA256 (important for large files)
- Sharding prevents directory bloat (max ~256 items per dir)
- Easy cleanup: delete unreferenced blobs
```

---

## 4. Core Implementation Details

### 4.1 Deterministic Render Engine

**Critical**: This is the most important component. Reproducibility depends on it.

```rust
// crates/ctx-core/src/render.rs
use anyhow::Result;
use std::collections::BTreeMap;

pub struct RenderEngine {
    storage: Arc<Storage>,
    token_estimator: Arc<TokenEstimator>,
    redactor: Arc<Redactor>,
}

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub pack_ids: Vec<String>,
    pub policy_overrides: Option<RenderPolicy>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderResult {
    pub budget_tokens: usize,
    pub token_estimate: usize,
    pub included: Vec<ArtifactSummary>,
    pub excluded: Vec<ExclusionInfo>,
    pub redactions: Vec<RedactionSummary>,
    pub render_hash: String,
    pub payload_text: Option<String>,
}

impl RenderEngine {
    pub async fn render(&self, req: RenderRequest) -> Result<RenderResult> {
        // 1. Load packs in stable order (by pack_ids order)
        let packs = self.load_packs_ordered(&req.pack_ids).await?;

        // 2. Collect all artifacts with stable ordering
        //    Within each pack: ORDER BY priority DESC, added_at ASC
        let mut ordered_artifacts = Vec::new();
        for pack in &packs {
            let pack_artifacts = self.storage
                .get_pack_artifacts(&pack.id)
                .await?;
            ordered_artifacts.extend(pack_artifacts);
        }

        // 3. Expand collections (md_dir, glob) deterministically
        //    CRITICAL: Sort expansion results lexicographically
        let expanded = self.expand_collections(ordered_artifacts).await?;

        // 4. Load content for each artifact
        let mut artifact_contents = Vec::new();
        for artifact in &expanded {
            let content = self.load_artifact_content(artifact).await?;
            artifact_contents.push((artifact.clone(), content));
        }

        // 5. Apply redaction (deterministic pattern order)
        let mut redactions = Vec::new();
        let redacted_contents: Vec<_> = artifact_contents
            .iter()
            .map(|(artifact, content)| {
                let (redacted, artifact_redactions) =
                    self.redactor.redact(&artifact.id, content);
                redactions.extend(artifact_redactions);
                (artifact, redacted)
            })
            .collect();

        // 6. Estimate tokens per artifact
        let token_estimates: Vec<_> = redacted_contents
            .iter()
            .map(|(artifact, content)| {
                let tokens = self.token_estimator.estimate(content);
                (artifact, content, tokens)
            })
            .collect();

        // 7. Apply budget (drop lowest priority first)
        let policy = self.resolve_policy(&packs, req.policy_overrides);
        let (included, excluded) = self.apply_budget(
            token_estimates,
            policy.budget_tokens,
        );

        // 8. Concatenate payload in stable order
        let payload = self.concatenate_payload(&included);

        // 9. Compute hashes
        let render_hash = self.compute_render_hash(&req, &policy, &included);
        let payload_hash = blake3::hash(payload.as_bytes()).to_hex();

        Ok(RenderResult {
            budget_tokens: policy.budget_tokens,
            token_estimate: included.iter().map(|i| i.tokens).sum(),
            included: included.iter().map(|i| i.to_summary()).collect(),
            excluded: excluded.iter().map(|e| e.to_info()).collect(),
            redactions: self.summarize_redactions(redactions),
            render_hash: render_hash.to_string(),
            payload_text: Some(payload),
        })
    }

    /// CRITICAL: Render hash must be deterministic
    fn compute_render_hash(
        &self,
        req: &RenderRequest,
        policy: &RenderPolicy,
        included: &[IncludedArtifact],
    ) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();

        // Hash pack IDs in order
        for pack_id in &req.pack_ids {
            hasher.update(pack_id.as_bytes());
        }

        // Hash policy
        let policy_json = serde_json::to_string(policy).unwrap();
        hasher.update(policy_json.as_bytes());

        // Hash artifact content hashes in order
        for artifact in included {
            hasher.update(artifact.content_hash.as_bytes());
        }

        hasher.finalize()
    }

    /// Expand collection artifacts into individual items
    async fn expand_collections(
        &self,
        artifacts: Vec<Artifact>,
    ) -> Result<Vec<Artifact>> {
        let mut expanded = Vec::new();

        for artifact in artifacts {
            match &artifact.artifact_type {
                ArtifactType::CollectionMdDir { path, max_files, exclude, recursive } => {
                    let mut files = self.scan_md_dir(path, *recursive, exclude)?;

                    // CRITICAL: Sort lexicographically for determinism
                    files.sort();

                    if let Some(max) = max_files {
                        files.truncate(*max);
                    }

                    for file_path in files {
                        expanded.push(self.create_file_artifact(&file_path).await?);
                    }
                }
                ArtifactType::CollectionGlob { pattern } => {
                    let mut matches = self.glob_match(pattern)?;

                    // CRITICAL: Sort for determinism
                    matches.sort();

                    for file_path in matches {
                        expanded.push(self.create_file_artifact(&file_path).await?);
                    }
                }
                _ => {
                    expanded.push(artifact);
                }
            }
        }

        Ok(expanded)
    }

    fn apply_budget(
        &self,
        artifacts: Vec<(&Artifact, &String, usize)>,
        budget: usize,
    ) -> (Vec<IncludedArtifact>, Vec<ExcludedArtifact>) {
        let mut included = Vec::new();
        let mut excluded = Vec::new();
        let mut total_tokens = 0;

        // Artifacts are already sorted by priority DESC, added_at ASC
        for (artifact, content, tokens) in artifacts {
            if total_tokens + tokens <= budget {
                included.push(IncludedArtifact {
                    artifact: artifact.clone(),
                    content: content.clone(),
                    tokens,
                    content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
                });
                total_tokens += tokens;
            } else {
                excluded.push(ExcludedArtifact {
                    artifact: artifact.clone(),
                    tokens,
                    reason: ExclusionReason::OverBudget,
                });
            }
        }

        (included, excluded)
    }
}
```

### 4.2 Source Handler Trait

```rust
// crates/ctx-sources/src/handler.rs
use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait SourceHandler: Send + Sync {
    /// Parse source URI into artifact metadata
    async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact>;

    /// Load content from source (called during render)
    async fn load(&self, artifact: &Artifact) -> Result<String>;

    /// Expand collection into individual artifacts (for collections only)
    async fn expand(&self, artifact: &Artifact) -> Result<Vec<Artifact>>;

    /// Check if this handler can handle the given URI
    fn can_handle(&self, uri: &str) -> bool;
}

#[derive(Debug, Clone, Default)]
pub struct SourceOptions {
    pub range: Option<(usize, usize)>,
    pub max_files: Option<usize>,
    pub exclude: Vec<String>,
    pub recursive: bool,
    pub base: Option<String>,
    pub head: Option<String>,
    pub capture: bool,
}

// Example: File handler
pub struct FileHandler;

#[async_trait]
impl SourceHandler for FileHandler {
    async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact> {
        let path = uri.strip_prefix("file:").unwrap();

        let artifact_type = if let Some((start, end)) = options.range {
            ArtifactType::FileRange {
                path: path.to_string(),
                start,
                end,
            }
        } else {
            ArtifactType::File {
                path: path.to_string(),
            }
        };

        // Read file, compute hash
        let content = tokio::fs::read_to_string(path).await?;
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        Ok(Artifact {
            id: uuid::Uuid::new_v4().to_string(),
            artifact_type,
            source_uri: uri.to_string(),
            content_hash,
            metadata: ArtifactMetadata {
                size_bytes: content.len(),
                mime_type: None,
                extra: serde_json::json!({}),
            },
            token_estimate: 0, // Will be estimated later
            created_at: OffsetDateTime::now_utc(),
        })
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        match &artifact.artifact_type {
            ArtifactType::File { path } => {
                tokio::fs::read_to_string(path).await
                    .map_err(Into::into)
            }
            ArtifactType::FileRange { path, start, end } => {
                let content = tokio::fs::read_to_string(path).await?;
                let lines: Vec<_> = content.lines().collect();
                Ok(lines[*start..=*end].join("\n"))
            }
            _ => anyhow::bail!("Unsupported artifact type for FileHandler"),
        }
    }

    async fn expand(&self, _artifact: &Artifact) -> Result<Vec<Artifact>> {
        // File is not a collection, return as-is
        Ok(vec![])
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("file:")
    }
}
```

### 4.3 Redaction Engine

```rust
// crates/ctx-security/src/redactor.rs
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref REDACTION_PATTERNS: Vec<RedactionPattern> = vec![
        RedactionPattern {
            name: "API_KEY".to_string(),
            pattern: Regex::new(r"(?i)api[_-]?key['\"\s:=]+([a-zA-Z0-9_-]{20,})").unwrap(),
        },
        RedactionPattern {
            name: "BEARER_TOKEN".to_string(),
            pattern: Regex::new(r"(?i)bearer\s+([a-zA-Z0-9_.\-]+)").unwrap(),
        },
        RedactionPattern {
            name: "AWS_ACCESS_KEY".to_string(),
            pattern: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        },
        RedactionPattern {
            name: "PRIVATE_KEY".to_string(),
            pattern: Regex::new(r"-----BEGIN\s+(?:RSA\s+)?PRIVATE KEY-----").unwrap(),
        },
        RedactionPattern {
            name: "JWT".to_string(),
            pattern: Regex::new(r"eyJ[a-zA-Z0-9_-]+\.eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+").unwrap(),
        },
        RedactionPattern {
            name: "GITHUB_TOKEN".to_string(),
            pattern: Regex::new(r"gh[ps]_[a-zA-Z0-9]{36,}").unwrap(),
        },
        // Add more patterns...
    ];
}

pub struct Redactor {
    custom_patterns: Vec<RedactionPattern>,
}

#[derive(Clone)]
struct RedactionPattern {
    name: String,
    pattern: Regex,
}

impl Redactor {
    pub fn new(custom_patterns: Vec<RedactionPattern>) -> Self {
        Self { custom_patterns }
    }

    pub fn redact(&self, artifact_id: &str, content: &str) -> (String, Vec<RedactionInfo>) {
        let mut result = content.to_string();
        let mut redactions = Vec::new();

        // Apply built-in patterns
        for pattern in REDACTION_PATTERNS.iter() {
            let matches = pattern.pattern.find_iter(&result).count();
            if matches > 0 {
                result = pattern.pattern.replace_all(
                    &result,
                    format!("[REDACTED:{}]", pattern.name)
                ).to_string();

                redactions.push(RedactionInfo {
                    artifact_id: artifact_id.to_string(),
                    redaction_type: pattern.name.clone(),
                    count: matches,
                });
            }
        }

        // Apply custom patterns
        for pattern in &self.custom_patterns {
            let matches = pattern.pattern.find_iter(&result).count();
            if matches > 0 {
                result = pattern.pattern.replace_all(
                    &result,
                    format!("[REDACTED:{}]", pattern.name)
                ).to_string();

                redactions.push(RedactionInfo {
                    artifact_id: artifact_id.to_string(),
                    redaction_type: pattern.name.clone(),
                    count: matches,
                });
            }
        }

        (result, redactions)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RedactionInfo {
    pub artifact_id: String,
    pub redaction_type: String,
    pub count: usize,
}
```

### 4.4 Token Estimation

```rust
// crates/ctx-tokens/src/estimator.rs
use tiktoken_rs::{cl100k_base, CoreBPE};
use std::sync::Arc;

pub struct TokenEstimator {
    bpe: Arc<CoreBPE>,
}

impl TokenEstimator {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            bpe: Arc::new(cl100k_base()?),
        })
    }

    pub fn estimate(&self, content: &str) -> usize {
        self.bpe.encode_with_special_tokens(content).len()
    }

    pub fn estimate_batch(&self, contents: &[String]) -> Vec<usize> {
        contents.iter()
            .map(|c| self.estimate(c))
            .collect()
    }
}

impl Default for TokenEstimator {
    fn default() -> Self {
        Self::new().expect("Failed to initialize token estimator")
    }
}
```

---

## 5. MCP Server Implementation

### 5.1 Axum HTTP Server

```rust
// crates/ctx-mcp/src/server.rs
use axum::{
    Router,
    routing::post,
    extract::State,
    Json,
};
use std::sync::Arc;

pub struct McpServer {
    db: Arc<Storage>,
    renderer: Arc<RenderEngine>,
    read_only: bool,
}

#[derive(Clone)]
struct AppState {
    server: Arc<McpServer>,
}

impl McpServer {
    pub async fn serve(
        self,
        host: &str,
        port: u16,
    ) -> anyhow::Result<()> {
        let app_state = AppState {
            server: Arc::new(self),
        };

        let app = Router::new()
            .route("/", post(handle_jsonrpc))
            .with_state(app_state);

        let addr = format!("{}:{}", host, port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        tracing::info!("MCP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn handle_jsonrpc(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    match req.method.as_str() {
        "tools/list" => {
            let tools = list_tools(state.server.read_only);
            Json(JsonRpcResponse::success(req.id, tools))
        }
        "tools/call" => {
            match call_tool(&state.server, &req.params).await {
                Ok(result) => Json(JsonRpcResponse::success(req.id, result)),
                Err(e) => Json(JsonRpcResponse::error(req.id, -32000, &e.to_string())),
            }
        }
        _ => Json(JsonRpcResponse::error(req.id, -32601, "Method not found")),
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: serde_json::Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
            }),
        }
    }
}
```

### 5.2 MCP Tools

```rust
// crates/ctx-mcp/src/tools.rs

async fn call_tool(
    server: &McpServer,
    params: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let tool_name = params["name"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
    let args = &params["arguments"];

    match tool_name {
        "ctx_packs_list" => {
            let packs = server.db.list_packs().await?;
            Ok(serde_json::to_value(packs)?)
        }
        "ctx_packs_get" => {
            let pack_name = args["pack"].as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pack parameter"))?;
            let pack = server.db.get_pack_by_name(pack_name).await?;
            Ok(serde_json::to_value(pack)?)
        }
        "ctx_packs_preview" => {
            let pack_ids: Vec<String> = serde_json::from_value(args["packs"].clone())?;
            let show_payload = args["show_payload"].as_bool().unwrap_or(false);

            let mut result = server.renderer.render(RenderRequest {
                pack_ids,
                policy_overrides: None,
            }).await?;

            if !show_payload {
                result.payload_text = None;
            }

            Ok(serde_json::to_value(result)?)
        }
        "ctx_packs_render" => {
            let result = if let Some(snapshot_id) = args["snapshot_id"].as_str() {
                // Load from snapshot
                server.load_snapshot_payload(snapshot_id).await?
            } else {
                // Render and auto-create snapshot
                let pack_ids: Vec<String> = serde_json::from_value(args["packs"].clone())?;
                let result = server.renderer.render(RenderRequest {
                    pack_ids,
                    policy_overrides: None,
                }).await?;

                // Create snapshot
                let snapshot = server.db.create_snapshot(&result).await?;

                serde_json::json!({
                    "snapshot_id": snapshot.id,
                    "render_hash": result.render_hash,
                    "payload_text": result.payload_text,
                })
            };

            Ok(result)
        }
        "ctx_packs_snapshot" => {
            let pack_ids: Vec<String> = serde_json::from_value(args["packs"].clone())?;
            let label = args["label"].as_str().map(String::from);

            let result = server.renderer.render(RenderRequest {
                pack_ids,
                policy_overrides: None,
            }).await?;

            let snapshot = server.db.create_snapshot_with_label(&result, label).await?;

            Ok(serde_json::json!({
                "snapshot_id": snapshot.id,
                "render_hash": result.render_hash,
            }))
        }
        _ => anyhow::bail!("Unknown tool: {}", tool_name),
    }
}

fn list_tools(read_only: bool) -> serde_json::Value {
    let mut tools = vec![
        tool_schema("ctx_packs_list", "List all context packs", serde_json::json!({})),
        tool_schema("ctx_packs_get", "Get pack details", serde_json::json!({
            "type": "object",
            "properties": {
                "pack": {"type": "string", "description": "Pack name or ID"}
            },
            "required": ["pack"]
        })),
        tool_schema("ctx_packs_preview", "Preview pack rendering", serde_json::json!({
            "type": "object",
            "properties": {
                "packs": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Pack IDs to render"
                },
                "show_payload": {"type": "boolean", "default": false}
            },
            "required": ["packs"]
        })),
        tool_schema("ctx_packs_render", "Render packs into LLM payload", serde_json::json!({
            "type": "object",
            "properties": {
                "packs": {"type": "array", "items": {"type": "string"}},
                "snapshot_id": {"type": "string"},
                "show_payload": {"type": "boolean", "default": true}
            }
        })),
        tool_schema("ctx_packs_snapshot", "Create snapshot of rendered packs", serde_json::json!({
            "type": "object",
            "properties": {
                "packs": {"type": "array", "items": {"type": "string"}},
                "label": {"type": "string"}
            },
            "required": ["packs"]
        })),
    ];

    if !read_only {
        // Add write tools in future
    }

    serde_json::json!({ "tools": tools })
}

fn tool_schema(name: &str, description: &str, input_schema: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}
```

---

## 6. Milestone Implementation Plan

### M1: Packs + Persistence (Weeks 1-2)

**Deliverables**:
- SQLite database with migrations
- Blob storage implementation
- Pack CRUD operations
- Source handlers: file, file_range, md, md_dir, glob, text
- CLI commands: create, list, show, add, remove

**Implementation Steps**:

1. **Project setup** (Day 1-2)
   ```bash
   cargo new --bin ctx
   cd ctx
   # Set up workspace structure
   mkdir -p crates/{ctx-cli,ctx-core,ctx-storage,ctx-sources,ctx-security,ctx-tokens,ctx-mcp}
   # Configure Cargo.toml workspace
   # Add dependencies
   ```

2. **Storage layer** (Day 3-5)
   - Implement `ctx-storage` crate
   - SQLite connection with sqlx
   - Migration runner
   - Blob store (content-addressable)
   - DB operations: create_pack, add_artifact, list_packs, etc.

3. **Core domain models** (Day 4-6)
   - Define structs in `ctx-core`
   - Implement serialization
   - Add validation logic

4. **Source handlers** (Day 7-10)
   - Implement `SourceHandler` trait
   - File handler
   - Collection handlers (md_dir, glob)
   - Text handler

5. **CLI implementation** (Day 9-12)
   - Clap CLI structure
   - Pack commands
   - Error handling
   - Output formatting

6. **Testing** (Day 13-14)
   - Unit tests for each component
   - Integration tests for CLI
   - Fixture creation

**Acceptance Criteria**:
```bash
# Build
cargo build --release

# Test commands
./target/release/ctx pack create test-pack
./target/release/ctx pack list
./target/release/ctx pack add test-pack file:./README.md
./target/release/ctx pack add test-pack 'file:./src/main.rs#L1-L50'
./target/release/ctx pack add test-pack 'md_dir:./docs' --recursive
./target/release/ctx pack add test-pack 'glob:**/*.rs'
./target/release/ctx pack add test-pack 'text:Hello world'
./target/release/ctx pack show test-pack
./target/release/ctx pack remove test-pack <artifact-id>

# Verify persistence
ls -la ~/.ctx/state.db
ls -la ~/.ctx/blobs/
```

---

### M2: Render + Preview + Snapshot (Weeks 3-4)

**Deliverables**:
- Deterministic render engine
- Token estimation with tiktoken
- Redaction engine
- Budget enforcement
- Preview command
- Snapshot storage

**Implementation Steps**:

1. **Token estimator** (Day 15-16)
   - Integrate tiktoken-rs
   - Caching strategy
   - Batch estimation

2. **Redaction engine** (Day 16-17)
   - Pattern definitions
   - Redaction logic
   - Reporting

3. **Render engine core** (Day 18-22) **CRITICAL**
   - Deterministic ordering
   - Collection expansion
   - Content loading
   - Concatenation
   - Hash computation
   - **EXTENSIVE TESTING**

4. **Budget enforcement** (Day 21-23)
   - Priority-based dropping
   - Exclusion tracking

5. **Preview command** (Day 24-25)
   - CLI implementation
   - Output formatting
   - Optional payload display

6. **Snapshot storage** (Day 26-27)
   - Snapshot creation
   - Payload storage in blobs
   - Snapshot retrieval

7. **Testing** (Day 28)
   - Determinism tests
   - Hash stability tests
   - Budget tests
   - Integration tests

**Acceptance Criteria**:
```bash
# Preview without payload
ctx pack preview test-pack --tokens

# Preview with payload
ctx pack preview test-pack --show-payload

# Multi-pack
ctx pack create styles
ctx pack add styles 'text:Use Rust idioms'
ctx pack preview test-pack --with-pack styles --show-payload

# Snapshot
ctx pack snapshot test-pack --name "v1.0"

# Reproducibility test
ctx pack preview test-pack --show-payload > out1.txt
ctx pack preview test-pack --show-payload > out2.txt
diff out1.txt out2.txt  # Must be identical
```

---

### M3: MCP Server (Weeks 5-6)

**Deliverables**:
- JSON-RPC 2.0 server (Axum)
- MCP tools: list, get, preview, render, snapshot
- Read-only mode enforcement
- Integration with render engine

**Implementation Steps**:

1. **JSON-RPC protocol** (Day 29-31)
   - Request/response types
   - Error handling
   - Protocol compliance

2. **Axum server** (Day 32-34)
   - HTTP endpoint
   - State management
   - Middleware (CORS, logging)

3. **MCP tools** (Day 35-38)
   - Tool definitions
   - Tool handlers
   - Schema generation

4. **Server command** (Day 39)
   - CLI integration
   - Configuration

5. **Testing** (Day 40-42)
   - Protocol tests
   - Tool tests
   - Integration tests with mock client
   - Invariant tests (preview == render)

**Acceptance Criteria**:
```bash
# Start server
ctx mcp serve --port 17373 &

# Test with curl
curl -X POST http://127.0.0.1:17373 -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list"
}'

curl -X POST http://127.0.0.1:17373 -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "ctx_packs_render",
    "arguments": {
      "packs": ["test-pack"],
      "show_payload": true
    }
  }
}'

# Integration with MCP client (Claude Code)
# Verify agent can call tools and get payloads
```

---

### M4: Hardening (Weeks 7-8)

**Deliverables**:
- Denylist implementation
- Git diff handler
- Command output handler
- Configuration system
- Documentation
- Performance optimization

**Implementation Steps**:

1. **Security features** (Day 43-45)
   - Denylist patterns
   - Path validation
   - Warning system

2. **Additional handlers** (Day 46-48)
   - Git handler (git2 library)
   - Command handler (tokio::process)

3. **Configuration** (Day 49-50)
   - Config file parsing
   - Defaults
   - Override logic

4. **Error handling** (Day 51-52)
   - Better error messages
   - User-friendly output
   - Suggestions

5. **Documentation** (Day 53-54)
   - README
   - Examples
   - Architecture docs

6. **Performance** (Day 55-56)
   - Benchmarking
   - Optimization
   - Large file handling

**Acceptance Criteria**:
```bash
# Security
ctx pack add test 'file:.env'  # Should warn/block
ctx pack add test 'file:.env' --allow-sensitive  # Should succeed

# Git
ctx pack add feature 'git:diff --base=main --head=HEAD'
ctx pack preview feature

# Command
ctx pack add debug 'cmd:ls -la' --capture
ctx pack preview debug --show-payload

# Config
cat ~/.ctx/config.toml
ctx pack preview test --tokens  # Uses config defaults

# Performance
ctx pack create large
ctx pack add large 'glob:**/*.rs'
time ctx pack preview large --tokens  # Should be fast
```

---

## 7. Testing Strategy

### 7.1 Unit Tests

```rust
// Example: Determinism test
#[tokio::test]
async fn test_render_determinism() {
    let engine = setup_test_engine().await;

    let req = RenderRequest {
        pack_ids: vec!["test-pack".to_string()],
        policy_overrides: None,
    };

    let result1 = engine.render(req.clone()).await.unwrap();
    let result2 = engine.render(req.clone()).await.unwrap();

    assert_eq!(result1.render_hash, result2.render_hash);
    assert_eq!(result1.payload_text, result2.payload_text);
}

// Property-based test with proptest
proptest! {
    #[test]
    fn test_render_hash_stability(artifacts in vec_artifact_strategy()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let engine = setup_test_engine().await;
            // Test that same inputs always produce same hash
        });
    }
}
```

### 7.2 Integration Tests

```bash
# tests/integration/test_e2e.sh
#!/bin/bash
set -euo pipefail

CTX="./target/release/ctx"
TEST_DIR=$(mktemp -d)

# Create test pack
$CTX pack create integration-test

# Add artifacts
$CTX pack add integration-test file:./README.md
$CTX pack add integration-test 'glob:src/**/*.rs'

# Preview
OUTPUT=$($CTX pack preview integration-test --tokens)
echo "$OUTPUT" | grep -q "estimated tokens"

# Snapshot
$CTX pack snapshot integration-test --name "test-snapshot"

# Verify reproducibility
$CTX pack preview integration-test --show-payload > "$TEST_DIR/out1.txt"
$CTX pack preview integration-test --show-payload > "$TEST_DIR/out2.txt"
diff "$TEST_DIR/out1.txt" "$TEST_DIR/out2.txt"

echo "✅ E2E tests passed"
```

### 7.3 Benchmarks

```rust
// benches/rendering.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_render_large_pack(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("render 1000 files", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Render pack with 1000 files
            })
        });
    });
}

criterion_group!(benches, bench_render_large_pack);
criterion_main!(benches);
```

---

## 8. Build & Distribution

### 8.1 Release Build

```bash
# Optimized release build
cargo build --release

# Strip binary (reduce size)
strip target/release/ctx

# Cross-compilation
cargo install cross
cross build --target x86_64-unknown-linux-gnu --release
cross build --target x86_64-apple-darwin --release
cross build --target aarch64-apple-darwin --release
cross build --target x86_64-pc-windows-gnu --release
```

### 8.2 GitHub Actions CI/CD

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check

  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release

  release:
    if: startsWith(github.ref, 'refs/tags/')
    needs: [test, build]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: ctx
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

## 9. Key Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| **Hash instability** | Medium | Critical | Extensive property-based tests, CI checks on every PR |
| **Async complexity** | Low | Medium | Use tokio best practices, thorough error handling |
| **Large file performance** | Medium | Medium | Streaming, lazy loading, configurable limits |
| **SQLite concurrency** | Low | Low | WAL mode, connection pooling |
| **Cross-platform paths** | Low | Medium | Use std::path::Path, test on all platforms |
| **Token estimation drift** | Low | Low | Clearly label as estimate, document limitations |

---

## 10. Development Environment Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install tools
cargo install cargo-watch
cargo install cargo-nextest  # Faster test runner
cargo install cargo-audit    # Security audits
cargo install cargo-deny     # License/dependency checks

# Setup project
mkdir ctx && cd ctx
cargo init --name ctx

# Development workflow
cargo watch -x check -x test  # Auto-rebuild on changes
cargo nextest run             # Run tests (faster)
cargo clippy --all-targets    # Linting
cargo fmt                     # Formatting
```

---

## Conclusion

This technical plan provides a complete roadmap for building `ctx` in Rust. The key to success:

1. **Deterministic rendering** - obsessive focus from day one
2. **Testing** - extensive unit, integration, and property-based tests
3. **Incremental delivery** - validate each milestone before moving forward
4. **Type safety** - leverage Rust's type system for correctness

**Next Steps**:
1. Create repository
2. Set up Cargo workspace
3. Begin M1 implementation
4. Daily progress tracking

**Estimated Timeline**: 8 weeks (1 engineer) or 5-6 weeks (2 engineers with clear module boundaries)
