use anyhow::Result;
use ctx_storage::Storage;

pub async fn handle(storage: &Storage) -> Result<()> {
    ctx_tui::run(storage.clone()).await
}
