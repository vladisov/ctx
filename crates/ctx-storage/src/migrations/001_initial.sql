-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS packs (
    pack_id TEXT PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    policies_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS artifacts (
    artifact_id TEXT PRIMARY KEY NOT NULL,
    type_json TEXT NOT NULL,
    source_uri TEXT NOT NULL,
    content_hash TEXT,
    meta_json TEXT NOT NULL,
    token_est INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS pack_items (
    pack_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (pack_id, artifact_id),
    FOREIGN KEY (pack_id) REFERENCES packs(pack_id) ON DELETE CASCADE,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(artifact_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pack_items_pack_ordering
    ON pack_items(pack_id, priority DESC, added_at ASC);
