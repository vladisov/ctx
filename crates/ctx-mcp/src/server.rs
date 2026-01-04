//! MCP server implementation

pub struct McpServer {
    // TODO: Add storage, renderer, config
}

impl McpServer {
    pub fn new() -> Self {
        // TODO: Implement in M3
        todo!("Implement McpServer::new")
    }

    pub async fn serve(&self, _host: &str, _port: u16) -> anyhow::Result<()> {
        // TODO: Implement in M3
        // - Set up Axum router
        // - Add JSON-RPC endpoint
        // - Start server
        todo!("Implement McpServer::serve")
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
