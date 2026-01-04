-- Initial schema for ctx
-- This migration creates the core tables for packs, artifacts, and snapshots

-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA strict = ON;

-- Packs: Named bundles of artifacts
CREATE TABLE packs (
    pack_id TEXT PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    policies_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_packs_name ON packs(name);
CREATE INDEX idx_packs_updated ON packs(updated_at DESC);

-- Artifacts: Individual pieces of context
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

CREATE INDEX idx_artifacts_hash ON artifacts(content_hash);
CREATE INDEX idx_artifacts_created ON artifacts(created_at DESC);

-- Pack Items: Artifacts in a pack (many-to-many)
CREATE TABLE pack_items (
    pack_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (pack_id, artifact_id),
    FOREIGN KEY (pack_id) REFERENCES packs(pack_id) ON DELETE CASCADE,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(artifact_id) ON DELETE CASCADE
) STRICT;

-- CRITICAL: This index enforces deterministic ordering for rendering
-- ORDER BY priority DESC, added_at ASC
CREATE INDEX idx_pack_items_pack_ordering
    ON pack_items(pack_id, priority DESC, added_at ASC);

-- Snapshots: Immutable records of rendered packs
CREATE TABLE snapshots (
    snapshot_id TEXT PRIMARY KEY NOT NULL,
    label TEXT,
    render_hash TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_snapshots_render_hash ON snapshots(render_hash);
CREATE INDEX idx_snapshots_created ON snapshots(created_at DESC);
CREATE INDEX idx_snapshots_label ON snapshots(label) WHERE label IS NOT NULL;

-- Snapshot Items: Artifacts in a snapshot
CREATE TABLE snapshot_items (
    snapshot_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    render_meta_json TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, artifact_id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_snapshot_items_snapshot ON snapshot_items(snapshot_id);
