mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
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

    // Initialize storage once (creates connection pool and runs migrations)
    let storage = Storage::new(None).await?;

    match cli.command {
        cli::Commands::Pack(pack_cmd) => commands::pack::handle(pack_cmd, &storage).await,
    }
}
