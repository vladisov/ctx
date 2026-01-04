//! ctx CLI - Main entry point

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ctx")]
#[command(about = "Repeatable context for LLM workflows", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage context packs
    Pack {
        #[command(subcommand)]
        command: PackCommands,
    },
    /// Start MCP server
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
}

#[derive(Subcommand)]
enum PackCommands {
    /// Create a new pack
    Create {
        /// Name of the pack
        name: String,
    },
    /// List all packs
    List,
    /// Show pack details
    Show {
        /// Pack name or ID
        pack: String,
    },
    /// Add artifact to pack
    Add {
        /// Pack name or ID
        pack: String,
        /// Source URI (e.g., file:path, glob:pattern, text:content)
        source: String,
        // TODO: Add options (--recursive, --max-files, etc.)
    },
    /// Remove artifact from pack
    Remove {
        /// Pack name or ID
        pack: String,
        /// Artifact ID
        artifact_id: String,
    },
    /// Preview pack rendering
    Preview {
        /// Pack name or ID
        pack: String,
        /// Additional packs to include
        #[arg(long = "with-pack")]
        with_pack: Vec<String>,
        /// Show token estimates
        #[arg(long)]
        tokens: bool,
        /// Show the full payload
        #[arg(long)]
        show_payload: bool,
    },
    /// Create snapshot of pack
    Snapshot {
        /// Pack name or ID
        pack: String,
        /// Additional packs to include
        #[arg(long = "with-pack")]
        with_pack: Vec<String>,
        /// Snapshot label/name
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum McpCommands {
    /// Start MCP server
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to bind to
        #[arg(long, default_value = "17373")]
        port: u16,
        /// Read-only mode (no pack creation/modification)
        #[arg(long, default_value = "true")]
        read_only: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { command } => handle_pack_command(command).await?,
        Commands::Mcp { command } => handle_mcp_command(command).await?,
    }

    Ok(())
}

async fn handle_pack_command(command: PackCommands) -> anyhow::Result<()> {
    match command {
        PackCommands::Create { name } => {
            println!("Creating pack: {}", name);
            // TODO: Implement in M1
            Ok(())
        }
        PackCommands::List => {
            println!("Listing packs...");
            // TODO: Implement in M1
            Ok(())
        }
        PackCommands::Show { pack } => {
            println!("Showing pack: {}", pack);
            // TODO: Implement in M1
            Ok(())
        }
        PackCommands::Add { pack, source } => {
            println!("Adding {} to pack {}", source, pack);
            // TODO: Implement in M1
            Ok(())
        }
        PackCommands::Remove { pack, artifact_id } => {
            println!("Removing {} from pack {}", artifact_id, pack);
            // TODO: Implement in M1
            Ok(())
        }
        PackCommands::Preview {
            pack,
            with_pack,
            tokens,
            show_payload,
        } => {
            println!("Previewing pack: {}", pack);
            if !with_pack.is_empty() {
                println!("With additional packs: {:?}", with_pack);
            }
            if tokens {
                println!("(showing token estimates)");
            }
            if show_payload {
                println!("(showing full payload)");
            }
            // TODO: Implement in M2
            Ok(())
        }
        PackCommands::Snapshot { pack, with_pack, name } => {
            println!("Creating snapshot of pack: {}", pack);
            if !with_pack.is_empty() {
                println!("With additional packs: {:?}", with_pack);
            }
            if let Some(label) = name {
                println!("Label: {}", label);
            }
            // TODO: Implement in M2
            Ok(())
        }
    }
}

async fn handle_mcp_command(command: McpCommands) -> anyhow::Result<()> {
    match command {
        McpCommands::Serve { host, port, read_only } => {
            println!("Starting MCP server on {}:{}", host, port);
            println!("Read-only mode: {}", read_only);
            // TODO: Implement in M3
            Ok(())
        }
    }
}
