use anyhow::{Result, bail};
use ctx_mcp::{McpServer, run_stdio};
use ctx_storage::Storage;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;

pub async fn handle(
    storage: &Storage,
    host: String,
    port: u16,
    read_only: bool,
    tunnel: bool,
) -> Result<()> {
    let db = Arc::new(storage.clone());

    let _ngrok_process = if tunnel {
        Some(start_ngrok_tunnel(port).await?)
    } else {
        None
    };

    eprintln!("Starting MCP server on {}:{}", host, port);
    McpServer::serve(db, &host, port, read_only).await?;
    Ok(())
}

pub async fn handle_stdio(storage: &Storage, read_only: bool) -> Result<()> {
    let db = Arc::new(storage.clone());
    run_stdio(db, read_only).await?;
    Ok(())
}

async fn start_ngrok_tunnel(port: u16) -> Result<Child> {
    // Check if ngrok is installed
    if Command::new("ngrok").arg("--version").output().is_err() {
        bail!("ngrok not found. Install it from https://ngrok.com/download");
    }

    eprintln!("Starting ngrok tunnel...");

    // Start ngrok
    let child = Command::new("ngrok")
        .args(["http", &port.to_string(), "--log", "stderr"])
        .spawn()?;

    // Wait for ngrok to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Query ngrok API for public URL
    match get_ngrok_url().await {
        Ok(url) => {
            print_tunnel_info(&url);
        }
        Err(e) => {
            eprintln!("Warning: Could not get ngrok URL: {}", e);
            eprintln!("Check http://localhost:4040 for the tunnel URL");
        }
    }

    Ok(child)
}

fn print_tunnel_info(url: &str) {
    eprintln!();
    eprintln!("══════════════════════════════════════════════════════════════");
    eprintln!("  ctx MCP Server - Public Tunnel Active");
    eprintln!("══════════════════════════════════════════════════════════════");
    eprintln!();
    eprintln!("  Public URL: {}", url);
    eprintln!();
    eprintln!("  REST API Endpoints:");
    eprintln!();
    eprintln!("  Read operations:");
    eprintln!("    GET  {}/api/packs                 List all packs", url);
    eprintln!(
        "    GET  {}/api/packs/:name           Get pack details",
        url
    );
    eprintln!(
        "    GET  {}/api/packs/:name/render    Get rendered content",
        url
    );
    eprintln!();
    eprintln!("  Write operations:");
    eprintln!(
        "    POST   {}/api/packs               Create pack (body: {{\"name\": \"...\", \"budget_tokens\": 128000}})",
        url
    );
    eprintln!("    DELETE {}/api/packs/:name         Delete pack", url);
    eprintln!("    POST   {}/api/packs/:name/artifacts  Add artifact", url);
    eprintln!();
    eprintln!("  ─────────────────────────────────────────────────────────────");
    eprintln!("  Copy this for AI tools:");
    eprintln!("  ─────────────────────────────────────────────────────────────");
    eprintln!();
    eprintln!("  This is a ctx context pack server. Available operations:");
    eprintln!();
    eprintln!("  Reading context:");
    eprintln!("  - GET {}/api/packs to list available packs", url);
    eprintln!("  - GET {}/api/packs/PACK_NAME/render to get content", url);
    eprintln!("    Returns JSON with 'content' field containing full context.");
    eprintln!();
    eprintln!("  Managing packs:");
    eprintln!(
        "  - POST {}/api/packs with {{\"name\": \"pack-name\"}} to create",
        url
    );
    eprintln!("  - DELETE {}/api/packs/PACK_NAME to delete", url);
    eprintln!(
        "  - POST {}/api/packs/PACK_NAME/artifacts with artifact type:",
        url
    );
    eprintln!("      {{\"type\": \"text\", \"content\": \"...\"}} for inline text");
    eprintln!("      {{\"type\": \"file\", \"path\": \"/path/to/file\"}} for files");
    eprintln!();
    eprintln!("══════════════════════════════════════════════════════════════");
    eprintln!();
}

async fn get_ngrok_url() -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:4040/api/tunnels")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let tunnels = resp["tunnels"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No tunnels found"))?;

    for tunnel in tunnels {
        if let Some(url) = tunnel["public_url"].as_str() {
            // Prefer https
            if url.starts_with("https://") {
                return Ok(url.to_string());
            }
        }
    }

    // Fall back to first tunnel
    tunnels
        .first()
        .and_then(|t| t["public_url"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No tunnel URL found"))
}
