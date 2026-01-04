//! Database operations

use crate::Result;

/// Main storage interface
pub struct Storage {
    // TODO: Add sqlx::SqlitePool
}

impl Storage {
    pub async fn new(_db_path: &str) -> Result<Self> {
        // TODO: Implement in M1
        // - Create SQLite connection pool
        // - Run migrations
        // - Return Storage instance
        todo!("Implement Storage::new")
    }

    // TODO: Implement CRUD operations:
    // - create_pack
    // - get_pack
    // - list_packs
    // - delete_pack
    // - create_artifact
    // - get_artifact
    // - add_artifact_to_pack
    // - remove_artifact_from_pack
    // - get_pack_artifacts (with ordering)
    // - create_snapshot
    // - get_snapshot
}
