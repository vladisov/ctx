use anyhow::Result;
use ctx_storage::Storage;
use ctx_mcp::McpServer;
use std::sync::Arc;

pub async fn handle(storage: &Storage, host: String, port: u16, read_only: bool) -> Result<()> {
    // For MCP, we need Arc<Storage>. 
    // Since `main.rs` creates `Storage` and passes `&Storage`, we might need to clone it.
    // Storage should be cheap to clone (Arc<Pool>).
    
    let db = Arc::new(storage.clone());
    
    println!("Starting MCP server on {}:{}", host, port);
    McpServer::serve(db, &host, port, read_only).await?;
    
    Ok(())
}
