mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use ctx_config::Config;
use ctx_storage::Storage;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = cli::Cli::parse();

    // Load config (creates default if not found)
    let config = Config::load()?;

    // Initialize storage (use custom data dir if provided)
    let db_path = cli.data_dir.as_ref().map(|dir| dir.join("state.db"));
    let storage = Storage::new(db_path).await?;

    match cli.command {
        cli::Commands::Pack(pack_cmd) => commands::pack::handle(pack_cmd, &storage, &config).await,
        cli::Commands::Mcp {
            stdio,
            port,
            host,
            read_only,
        } => {
            let read_only = read_only || config.mcp.read_only;
            if stdio {
                commands::mcp::handle_stdio(&storage, read_only).await
            } else {
                let port = port.unwrap_or(config.mcp.port);
                let host = host.unwrap_or(config.mcp.host);
                commands::mcp::handle(&storage, host, port, read_only).await
            }
        }
        cli::Commands::Ui => commands::ui::handle(&storage).await,
    }
}
