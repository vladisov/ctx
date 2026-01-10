use anyhow::Result;
use ctx_mcp::{run_stdio, McpServer};
use ctx_storage::Storage;
use std::sync::Arc;

pub async fn handle(storage: &Storage, host: String, port: u16, read_only: bool) -> Result<()> {
    let db = Arc::new(storage.clone());
    eprintln!("Starting MCP server on {}:{}", host, port);
    McpServer::serve(db, &host, port, read_only).await?;
    Ok(())
}

pub async fn handle_stdio(storage: &Storage, read_only: bool) -> Result<()> {
    let db = Arc::new(storage.clone());
    run_stdio(db, read_only).await?;
    Ok(())
}
