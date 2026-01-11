use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ctx")]
#[command(about = "Context management for LLMs", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Override data directory (for testing)
    #[arg(long, env = "CTX_DATA_DIR", global = true)]
    pub data_dir: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize ctx.toml in current directory
    Init {
        /// Import existing packs into ctx.toml
        #[arg(long)]
        import: Vec<String>,
    },

    /// Manage context packs
    #[command(subcommand)]
    Pack(PackCommands),

    /// Start MCP server
    Mcp {
        /// Use stdio transport (for Claude Code integration)
        #[arg(long)]
        stdio: bool,

        #[arg(long)]
        port: Option<u16>,

        #[arg(long)]
        host: Option<String>,

        #[arg(long)]
        read_only: bool,
    },

    /// Launch interactive UI
    Ui {
        /// Launch web UI instead of terminal UI
        #[arg(long)]
        web: bool,

        /// Port for web UI (default: 17380)
        #[arg(long, default_value = "17380")]
        port: u16,
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

    /// Delete a pack
    Delete {
        /// Pack name or ID
        pack: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Sync packs from ctx.toml to database
    Sync,

    /// Save pack(s) to ctx.toml
    Save {
        /// Pack name(s) to save (or --all)
        #[arg(required_unless_present = "all")]
        packs: Vec<String>,

        /// Save all packs
        #[arg(long)]
        all: bool,
    },
}
