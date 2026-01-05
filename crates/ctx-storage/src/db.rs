use ctx_core::{Artifact, Error, Pack, Result, Snapshot};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::PathBuf;
use std::str::FromStr;

use crate::blob::BlobStore;
use crate::models::PackItem;

#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
    blob_store: BlobStore,
}

impl Storage {
    pub async fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let path = db_path.unwrap_or_else(|| {
            let dirs = directories::ProjectDirs::from("com", "ctx", "ctx").unwrap();
            let data_dir = dirs.data_dir();
            std::fs::create_dir_all(data_dir).unwrap();
            data_dir.join("state.db")
        });

        // Ensure parent directory exists (important for custom paths)
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Database(format!("Failed to create data directory: {}", e)))?;
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
            .map_err(|e| Error::Database(e.to_string()))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let blob_store = BlobStore::new(None);

        let storage = Self { pool, blob_store };
        storage.run_migrations().await?;

        Ok(storage)
    }

    async fn run_migrations(&self) -> Result<()> {
        // Create migrations tracking table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create migrations table: {}", e)))?;

        // Check if migration 1 has been applied
        let applied: Option<i64> =
            sqlx::query_scalar("SELECT version FROM _migrations WHERE version = 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Error::Database(format!("Failed to check migration status: {}", e)))?;

        if applied.is_none() {
            // Run migration 1
            let migration_sql = include_str!("migrations/001_initial.sql");

            sqlx::query(migration_sql)
                .execute(&self.pool)
                .await
                .map_err(|e| Error::Database(format!("Failed to run migration 001: {}", e)))?;

            // Mark as applied
            sqlx::query("INSERT INTO _migrations (version, applied_at) VALUES (1, ?)")
                .bind(time::OffsetDateTime::now_utc().unix_timestamp())
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    Error::Database(format!("Failed to mark migration as applied: {}", e))
                })?;
        }

        Ok(())
    }

    // Pack operations

    /// Get pack by name or ID in a single query
    pub async fn get_pack(&self, name_or_id: &str) -> Result<Pack> {
        let row = sqlx::query(
            "SELECT pack_id, name, policies_json, created_at, updated_at
             FROM packs
             WHERE pack_id = ? OR name = ?
             LIMIT 1",
        )
        .bind(name_or_id)
        .bind(name_or_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to fetch pack '{}': {}", name_or_id, e)))?
        .ok_or_else(|| Error::PackNotFound(name_or_id.to_string()))?;

        self.row_to_pack(row)
    }

    pub async fn create_pack(&self, pack: &Pack) -> Result<()> {
        let policies_json = serde_json::to_string(&pack.policies)?;

        sqlx::query(
            "INSERT INTO packs (pack_id, name, policies_json, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&pack.id)
        .bind(&pack.name)
        .bind(&policies_json)
        .bind(pack.created_at.unix_timestamp())
        .bind(pack.updated_at.unix_timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                Error::PackAlreadyExists(pack.name.clone())
            } else {
                Error::Database(e.to_string())
            }
        })?;

        Ok(())
    }

    pub async fn get_pack_by_name(&self, name: &str) -> Result<Pack> {
        let row = sqlx::query(
            "SELECT pack_id, name, policies_json, created_at, updated_at FROM packs WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to fetch pack by name '{}': {}", name, e)))?
        .ok_or_else(|| Error::PackNotFound(name.to_string()))?;

        self.row_to_pack(row)
    }

    pub async fn get_pack_by_id(&self, id: &str) -> Result<Pack> {
        let row = sqlx::query(
            "SELECT pack_id, name, policies_json, created_at, updated_at FROM packs WHERE pack_id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to fetch pack by id '{}': {}", id, e)))?
        .ok_or_else(|| Error::PackNotFound(id.to_string()))?;

        self.row_to_pack(row)
    }

    pub async fn list_packs(&self) -> Result<Vec<Pack>> {
        let rows = sqlx::query(
            "SELECT pack_id, name, policies_json, created_at, updated_at FROM packs ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to list packs: {}", e)))?;

        rows.into_iter().map(|row| self.row_to_pack(row)).collect()
    }

    // Artifact operations

    /// Create artifact and store its content in blob storage
    pub async fn create_artifact_with_content(
        &self,
        artifact: &Artifact,
        content: &str,
    ) -> Result<String> {
        // Store content in blob storage
        let content_hash = self.blob_store.store(content.as_bytes()).await?;

        // Create artifact with the hash
        let mut artifact_with_hash = artifact.clone();
        artifact_with_hash.content_hash = Some(content_hash.clone());

        self.create_artifact(&artifact_with_hash).await?;

        Ok(content_hash)
    }

    pub async fn create_artifact(&self, artifact: &Artifact) -> Result<()> {
        let type_json = serde_json::to_string(&artifact.artifact_type)?;
        let meta_json = serde_json::to_string(&artifact.metadata)?;

        sqlx::query(
            "INSERT INTO artifacts (artifact_id, type_json, source_uri, content_hash, meta_json, token_est, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&artifact.id)
        .bind(&type_json)
        .bind(&artifact.source_uri)
        .bind(&artifact.content_hash)
        .bind(&meta_json)
        .bind(artifact.token_estimate as i64)
        .bind(artifact.created_at.unix_timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to create artifact: {}", e)))?;

        Ok(())
    }

    /// Load artifact content from blob storage
    pub async fn load_artifact_content(&self, artifact: &Artifact) -> Result<String> {
        let content_hash = artifact
            .content_hash
            .as_ref()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Artifact has no content hash")))?;

        let content_bytes = self.blob_store.retrieve(content_hash).await?;

        String::from_utf8(content_bytes)
            .map_err(|e| Error::Other(anyhow::anyhow!("Invalid UTF-8 in artifact content: {}", e)))
    }

    pub async fn get_artifact(&self, id: &str) -> Result<Artifact> {
        let row = sqlx::query(
            "SELECT artifact_id, type_json, source_uri, content_hash, meta_json, token_est, created_at
             FROM artifacts WHERE artifact_id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::ArtifactNotFound(id.to_string()))?;

        self.row_to_artifact(row)
    }

    fn row_to_pack(&self, row: sqlx::sqlite::SqliteRow) -> Result<Pack> {
        let id: String = row.get("pack_id");
        let name: String = row.get("name");
        let policies_json: String = row.get("policies_json");
        let created_at: i64 = row.get("created_at");
        let updated_at: i64 = row.get("updated_at");

        Ok(Pack {
            id,
            name,
            policies: serde_json::from_str(&policies_json).map_err(|e| {
                Error::Other(anyhow::anyhow!("Failed to parse policies JSON: {}", e))
            })?,
            created_at: time::OffsetDateTime::from_unix_timestamp(created_at)
                .map_err(|e| Error::Other(e.into()))?,
            updated_at: time::OffsetDateTime::from_unix_timestamp(updated_at)
                .map_err(|e| Error::Other(e.into()))?,
        })
    }

    fn row_to_artifact(&self, row: sqlx::sqlite::SqliteRow) -> Result<Artifact> {
        let id: String = row.get("artifact_id");
        let type_json: String = row.get("type_json");
        let source_uri: String = row.get("source_uri");
        let content_hash: Option<String> = row.get("content_hash");
        let meta_json: String = row.get("meta_json");
        let token_est: i64 = row.get("token_est");
        let created_at: i64 = row.get("created_at");

        Ok(Artifact {
            id,
            artifact_type: serde_json::from_str(&type_json).map_err(|e| {
                Error::Other(anyhow::anyhow!("Failed to parse artifact type JSON: {}", e))
            })?,
            source_uri,
            content_hash,
            metadata: serde_json::from_str(&meta_json).map_err(|e| {
                Error::Other(anyhow::anyhow!("Failed to parse metadata JSON: {}", e))
            })?,
            token_estimate: token_est as usize,
            created_at: time::OffsetDateTime::from_unix_timestamp(created_at)
                .map_err(|e| Error::Other(e.into()))?,
        })
    }

    // Pack-Artifact association operations

    /// Add artifact to pack with content, using a transaction for atomicity
    pub async fn add_artifact_to_pack_with_content(
        &self,
        pack_id: &str,
        artifact: &Artifact,
        content: &str,
        priority: i64,
    ) -> Result<String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(format!("Failed to begin transaction: {}", e)))?;

        // Store content in blob storage
        let content_hash = self.blob_store.store(content.as_bytes()).await?;

        // Create artifact with the hash
        let mut artifact_with_hash = artifact.clone();
        artifact_with_hash.content_hash = Some(content_hash.clone());

        // Insert artifact
        let type_json = serde_json::to_string(&artifact_with_hash.artifact_type)?;
        let meta_json = serde_json::to_string(&artifact_with_hash.metadata)?;

        sqlx::query(
            "INSERT INTO artifacts (artifact_id, type_json, source_uri, content_hash, meta_json, token_est, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&artifact_with_hash.id)
        .bind(&type_json)
        .bind(&artifact_with_hash.source_uri)
        .bind(&artifact_with_hash.content_hash)
        .bind(&meta_json)
        .bind(artifact_with_hash.token_estimate as i64)
        .bind(artifact_with_hash.created_at.unix_timestamp())
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(format!("Failed to create artifact in transaction: {}", e)))?;

        // Add to pack
        let added_at = time::OffsetDateTime::now_utc();
        sqlx::query(
            "INSERT INTO pack_items (pack_id, artifact_id, priority, added_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(pack_id)
        .bind(&artifact_with_hash.id)
        .bind(priority)
        .bind(added_at.unix_timestamp())
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            Error::Database(format!(
                "Failed to add artifact to pack in transaction: {}",
                e
            ))
        })?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| Error::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(content_hash)
    }

    pub async fn add_artifact_to_pack(
        &self,
        pack_id: &str,
        artifact_id: &str,
        priority: i64,
    ) -> Result<()> {
        let added_at = time::OffsetDateTime::now_utc();

        sqlx::query(
            "INSERT INTO pack_items (pack_id, artifact_id, priority, added_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(pack_id)
        .bind(artifact_id)
        .bind(priority)
        .bind(added_at.unix_timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("Failed to add artifact to pack: {}", e)))?;

        Ok(())
    }

    pub async fn remove_artifact_from_pack(&self, pack_id: &str, artifact_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM pack_items WHERE pack_id = ? AND artifact_id = ?")
            .bind(pack_id)
            .bind(artifact_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(Error::ArtifactNotFound(artifact_id.to_string()));
        }

        Ok(())
    }

    pub async fn get_pack_artifacts(&self, pack_id: &str) -> Result<Vec<PackItem>> {
        let rows = sqlx::query(
            "SELECT a.artifact_id, a.type_json, a.source_uri, a.content_hash, a.meta_json,
                    a.token_est, a.created_at, pi.priority, pi.added_at
             FROM artifacts a
             JOIN pack_items pi ON a.artifact_id = pi.artifact_id
             WHERE pi.pack_id = ?
             ORDER BY pi.priority DESC, pi.added_at ASC",
        )
        .bind(pack_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let mut items = Vec::new();
        for row in rows {
            // Extract priority and added_at first (before consuming row)
            let priority: i64 = row.get("priority");
            let added_at: i64 = row.get("added_at");

            // Now extract artifact (this consumes the row)
            let artifact = self.row_to_artifact(row)?;

            items.push(PackItem {
                pack_id: pack_id.to_string(),
                artifact,
                priority,
                added_at: time::OffsetDateTime::from_unix_timestamp(added_at)
                    .map_err(|e| Error::Other(e.into()))?,
            });
        }

        Ok(items)
    }

    // Snapshot operations
    pub async fn create_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        sqlx::query(
            "INSERT INTO snapshots (snapshot_id, label, render_hash, payload_hash, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&snapshot.id)
        .bind(&snapshot.label)
        .bind(&snapshot.render_hash)
        .bind(&snapshot.payload_hash)
        .bind(snapshot.created_at.unix_timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn get_snapshot(&self, id: &str) -> Result<Snapshot> {
        let row = sqlx::query(
            "SELECT snapshot_id, label, render_hash, payload_hash, created_at
             FROM snapshots WHERE snapshot_id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?
        .ok_or_else(|| Error::SnapshotNotFound(id.to_string()))?;

        let id: String = row.get("snapshot_id");
        let label: Option<String> = row.get("label");
        let render_hash: String = row.get("render_hash");
        let payload_hash: String = row.get("payload_hash");
        let created_at: i64 = row.get("created_at");

        Ok(Snapshot {
            id,
            label,
            render_hash,
            payload_hash,
            created_at: time::OffsetDateTime::from_unix_timestamp(created_at)
                .map_err(|e| Error::Other(e.into()))?,
        })
    }

    /// Delete a pack and all its associations (artifacts remain for deduplication)
    pub async fn delete_pack(&self, pack_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM packs WHERE pack_id = ?")
            .bind(pack_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(Error::PackNotFound(pack_id.to_string()));
        }

        Ok(())
    }

    /// List all snapshots, optionally filtered by render_hash
    pub async fn list_snapshots(&self, render_hash: Option<&str>) -> Result<Vec<Snapshot>> {
        let rows = if let Some(hash) = render_hash {
            sqlx::query(
                "SELECT snapshot_id, label, render_hash, payload_hash, created_at
                 FROM snapshots WHERE render_hash = ? ORDER BY created_at DESC",
            )
            .bind(hash)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                "SELECT snapshot_id, label, render_hash, payload_hash, created_at
                 FROM snapshots ORDER BY created_at DESC",
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| Error::Database(e.to_string()))?;

        let mut snapshots = Vec::new();
        for row in rows {
            let id: String = row.get("snapshot_id");
            let label: Option<String> = row.get("label");
            let render_hash: String = row.get("render_hash");
            let payload_hash: String = row.get("payload_hash");
            let created_at: i64 = row.get("created_at");

            snapshots.push(Snapshot {
                id,
                label,
                render_hash,
                payload_hash,
                created_at: time::OffsetDateTime::from_unix_timestamp(created_at)
                    .map_err(|e| Error::Other(e.into()))?,
            });
        }

        Ok(snapshots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctx_core::{Artifact, ArtifactType, Pack, RenderPolicy, Snapshot};

    async fn create_test_storage() -> Storage {
        let test_dir =
            std::env::temp_dir().join(format!("ctx-storage-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("test.db");
        Storage::new(Some(db_path)).await.unwrap()
    }

    #[tokio::test]
    async fn test_pack_crud() {
        let storage = create_test_storage().await;

        // Create pack
        let pack = Pack::new("test-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        // Get pack by name
        let retrieved = storage.get_pack_by_name("test-pack").await.unwrap();
        assert_eq!(retrieved.id, pack.id);
        assert_eq!(retrieved.name, "test-pack");

        // Get pack by ID
        let retrieved_by_id = storage.get_pack_by_id(&pack.id).await.unwrap();
        assert_eq!(retrieved_by_id.name, "test-pack");

        // List packs
        let packs = storage.list_packs().await.unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "test-pack");
    }

    #[tokio::test]
    async fn test_pack_already_exists() {
        let storage = create_test_storage().await;

        let pack = Pack::new("duplicate-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        // Try to create again with same name
        let pack2 = Pack::new("duplicate-pack".to_string(), RenderPolicy::default());
        let result = storage.create_pack(&pack2).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::PackAlreadyExists(_)));
    }

    #[tokio::test]
    async fn test_artifact_with_content() {
        let storage = create_test_storage().await;

        // Create artifact
        let artifact = Artifact::new(
            ArtifactType::Text {
                content: "test content".to_string(),
            },
            "text:test".to_string(),
        );
        let content = "Hello, world!";

        // Store with content
        let content_hash = storage
            .create_artifact_with_content(&artifact, content)
            .await
            .unwrap();
        assert!(!content_hash.is_empty());

        // Retrieve artifact
        let retrieved = storage.get_artifact(&artifact.id).await.unwrap();
        assert_eq!(retrieved.id, artifact.id);
        assert_eq!(retrieved.content_hash, Some(content_hash.clone()));

        // Load content
        let loaded_content = storage.load_artifact_content(&retrieved).await.unwrap();
        assert_eq!(loaded_content, content);
    }

    #[tokio::test]
    async fn test_pack_artifact_association() {
        let storage = create_test_storage().await;

        // Create pack
        let pack = Pack::new("test-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        // Create artifacts
        let artifact1 = Artifact::new(
            ArtifactType::Text {
                content: "content1".to_string(),
            },
            "text:1".to_string(),
        );
        let artifact2 = Artifact::new(
            ArtifactType::Text {
                content: "content2".to_string(),
            },
            "text:2".to_string(),
        );

        storage.create_artifact(&artifact1).await.unwrap();
        storage.create_artifact(&artifact2).await.unwrap();

        // Add to pack with different priorities
        storage
            .add_artifact_to_pack(&pack.id, &artifact1.id, 10)
            .await
            .unwrap();
        storage
            .add_artifact_to_pack(&pack.id, &artifact2.id, 5)
            .await
            .unwrap();

        // Get pack artifacts (should be sorted by priority DESC)
        let items = storage.get_pack_artifacts(&pack.id).await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].artifact.id, artifact1.id); // priority 10 first
        assert_eq!(items[1].artifact.id, artifact2.id); // priority 5 second

        // Remove artifact
        storage
            .remove_artifact_from_pack(&pack.id, &artifact1.id)
            .await
            .unwrap();
        let items = storage.get_pack_artifacts(&pack.id).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].artifact.id, artifact2.id);
    }

    #[tokio::test]
    async fn test_add_artifact_with_content_transactional() {
        let storage = create_test_storage().await;

        // Create pack
        let pack = Pack::new("test-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        // Add artifact with content (atomic operation)
        let artifact = Artifact::new(
            ArtifactType::File {
                path: "/test/file.txt".to_string(),
            },
            "file:/test/file.txt".to_string(),
        );
        let content = "File content here";

        let content_hash = storage
            .add_artifact_to_pack_with_content(&pack.id, &artifact, content, 0)
            .await
            .unwrap();

        // Verify artifact was created
        let retrieved = storage.get_artifact(&artifact.id).await.unwrap();
        assert_eq!(retrieved.content_hash, Some(content_hash));

        // Verify it's in the pack
        let items = storage.get_pack_artifacts(&pack.id).await.unwrap();
        assert_eq!(items.len(), 1);

        // Verify content can be loaded
        let loaded = storage.load_artifact_content(&retrieved).await.unwrap();
        assert_eq!(loaded, content);
    }

    #[tokio::test]
    async fn test_snapshot_operations() {
        let storage = create_test_storage().await;

        // Create snapshot
        let snapshot = Snapshot::new(
            "render-hash-123".to_string(),
            "payload-hash-456".to_string(),
            Some("v1.0".to_string()),
        );
        storage.create_snapshot(&snapshot).await.unwrap();

        // Retrieve snapshot
        let retrieved = storage.get_snapshot(&snapshot.id).await.unwrap();
        assert_eq!(retrieved.id, snapshot.id);
        assert_eq!(retrieved.label, Some("v1.0".to_string()));
        assert_eq!(retrieved.render_hash, "render-hash-123");
        assert_eq!(retrieved.payload_hash, "payload-hash-456");
    }

    #[tokio::test]
    async fn test_snapshot_not_found() {
        let storage = create_test_storage().await;

        let result = storage.get_snapshot("nonexistent-id").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::SnapshotNotFound(_)));
    }

    #[tokio::test]
    async fn test_multiple_packs_ordering() {
        let storage = create_test_storage().await;

        // Create multiple packs
        let pack_a = Pack::new("aaa-pack".to_string(), RenderPolicy::default());
        let pack_b = Pack::new("zzz-pack".to_string(), RenderPolicy::default());
        let pack_c = Pack::new("mmm-pack".to_string(), RenderPolicy::default());

        storage.create_pack(&pack_b).await.unwrap();
        storage.create_pack(&pack_a).await.unwrap();
        storage.create_pack(&pack_c).await.unwrap();

        // List should be alphabetically sorted
        let packs = storage.list_packs().await.unwrap();
        assert_eq!(packs.len(), 3);
        assert_eq!(packs[0].name, "aaa-pack");
        assert_eq!(packs[1].name, "mmm-pack");
        assert_eq!(packs[2].name, "zzz-pack");
    }

    #[tokio::test]
    async fn test_migrations_idempotent() {
        let test_dir =
            std::env::temp_dir().join(format!("ctx-storage-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("test.db");

        // Create storage (runs migrations)
        let storage1 = Storage::new(Some(db_path.clone())).await.unwrap();
        drop(storage1);

        // Create again with same DB (should not fail)
        let storage2 = Storage::new(Some(db_path.clone())).await.unwrap();

        // Verify DB is functional
        let pack = Pack::new("test-pack".to_string(), RenderPolicy::default());
        storage2.create_pack(&pack).await.unwrap();

        let packs = storage2.list_packs().await.unwrap();
        assert_eq!(packs.len(), 1);
    }
}
