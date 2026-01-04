use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ctx")]
#[command(about = "Context management for LLMs", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage context packs
    #[command(subcommand)]
    Pack(PackCommands),

    /// Start MCP server
    Mcp {
        #[arg(long)]
        port: Option<u16>,

        #[arg(long)]
        host: Option<String>,

        #[arg(long)]
        read_only: bool,
    },
}

#[derive(Subcommand)]
pub enum PackCommands {
    /// Create a new pack
    Create {
        /// Name of the pack
        name: String,

        /// Token budget (default from config: 128000)
        #[arg(long)]
        tokens: Option<usize>,
    },

    /// List all packs
    List,

    /// Show pack details
    Show {
        /// Pack name or ID
        pack: String,
    },

    /// Add an artifact to a pack
    Add {
        /// Pack name or ID
        pack: String,

        /// Source URI (e.g., file:path, text:content, glob:pattern)
        source: String,

        /// Priority (higher = included first when over budget)
        #[arg(long, default_value = "0")]
        priority: i64,

        /// For file ranges: start line (1-indexed)
        #[arg(long)]
        start: Option<usize>,

        /// For file ranges: end line (1-indexed)
        #[arg(long)]
        end: Option<usize>,

        /// For md_dir: maximum number of files
        #[arg(long)]
        max_files: Option<usize>,

        /// For md_dir: patterns to exclude
        #[arg(long)]
        exclude: Vec<String>,

        /// For md_dir: recursive scan
        #[arg(long)]
        recursive: bool,
    },

    /// Remove an artifact from a pack
    Remove {
        /// Pack name or ID
        pack: String,

        /// Artifact ID to remove
        artifact_id: String,
    },

    /// Preview pack rendering
    Preview {
        /// Pack name or ID
        pack: String,

        /// Show token counts
        #[arg(long)]
        tokens: bool,

        /// Show redaction details
        #[arg(long)]
        redactions: bool,

        /// Show the rendered payload
        #[arg(long)]
        show_payload: bool,
    },

    /// Create a snapshot of a pack
    Snapshot {
        /// Pack name or ID
        pack: String,

        /// Optional label for the snapshot
        #[arg(long)]
        label: Option<String>,
    },
}
