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
}

#[derive(Subcommand)]
pub enum PackCommands {
    /// Create a new pack
    Create {
        /// Name of the pack
        name: String,

        /// Token budget (default: 128000)
        #[arg(long, default_value = "128000")]
        tokens: usize,
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
}
