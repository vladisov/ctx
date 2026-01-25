use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

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
    // ===== Quick workflow =====
    /// Quick context: file + related files to clipboard
    #[command(name = "@")]
    Quick {
        /// File to build context from
        file: std::path::PathBuf,

        /// Output to stdout instead of clipboard
        #[arg(long, short)]
        output: bool,

        /// Max related files to include (default: 5)
        #[arg(long, short = 'n', default_value = "5")]
        max_related: usize,
    },

    // ===== Pack management =====
    /// Create a new pack
    Create {
        /// Name of the pack
        name: String,

        /// Token budget (default: 128000)
        #[arg(long)]
        tokens: Option<usize>,
    },

    /// Add source to a pack
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

        /// Also add related files (based on git history and imports)
        #[arg(long, short = 'r')]
        with_related: bool,

        /// Max related files to add (default: 5)
        #[arg(long, default_value = "5")]
        related_max: usize,
    },

    /// Remove artifact from a pack
    Rm {
        /// Pack name or ID
        pack: String,

        /// Artifact ID to remove
        artifact_id: String,
    },

    /// List all packs
    Ls,

    /// Show pack details
    Show {
        /// Pack name or ID
        pack: String,
    },

    /// Preview pack rendering
    Preview {
        /// Pack name or ID
        pack: String,

        /// Show token counts per artifact
        #[arg(long)]
        tokens: bool,

        /// Show redaction details
        #[arg(long)]
        redactions: bool,

        /// Show the full rendered payload
        #[arg(long, short)]
        payload: bool,
    },

    /// Copy pack to clipboard
    Cp {
        /// Pack name or ID
        pack: String,
    },

    /// Delete a pack
    Delete {
        /// Pack name or ID
        pack: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Check pack completeness (find missing dependencies)
    Lint {
        /// Pack name or ID
        pack: String,

        /// Auto-fix by adding missing files
        #[arg(long)]
        fix: bool,
    },

    // ===== Discovery =====
    /// Suggest related files
    Suggest {
        /// File to find suggestions for
        file: std::path::PathBuf,

        /// Maximum number of suggestions
        #[arg(long, short = 'n', default_value = "10")]
        max: usize,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    // ===== Project config =====
    /// Initialize ctx.toml in current directory
    Init {
        /// Import existing packs into ctx.toml
        #[arg(long)]
        import: Vec<String>,
    },

    /// Sync packs from ctx.toml
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


    // ===== Integrations =====
    /// Install ctx integration into other tools
    Install {
        /// Tools to install (claude, opencode, antigravity)
        #[arg(required = true, value_delimiter = ',', num_args = 1..)]
        targets: Vec<InstallTarget>,
    },

    // ===== Services =====
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

        /// Start ngrok tunnel for public access
        #[arg(long)]
        tunnel: bool,
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

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

impl Cli {
    /// Generate shell completions and write to stdout
    pub fn print_completions(shell: Shell) {
        let mut cmd = Self::command();
        clap_complete::generate(shell, &mut cmd, "ctx", &mut std::io::stdout());
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum InstallTarget {
    Claude,
    Opencode,
    Antigravity,
}
