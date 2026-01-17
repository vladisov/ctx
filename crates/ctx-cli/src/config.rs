use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Global Config (~/.ctx/config.toml)
// ============================================================================

/// Simple configuration for ctx
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_budget")]
    pub budget_tokens: usize,

    #[serde(default)]
    pub denylist: DenylistConfig,

    #[serde(default)]
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenylistConfig {
    #[serde(default = "default_patterns")]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default)]
    pub read_only: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            budget_tokens: default_budget(),
            denylist: DenylistConfig::default(),
            mcp: McpConfig::default(),
        }
    }
}

impl Default for DenylistConfig {
    fn default() -> Self {
        Self {
            patterns: default_patterns(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            read_only: false,
        }
    }
}

fn default_budget() -> usize {
    128_000
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    17373
}

fn default_patterns() -> Vec<String> {
    vec![
        "**/.env*".to_string(),
        "**/.aws/**".to_string(),
        "**/secrets/**".to_string(),
        "**/*_rsa".to_string(),
        "**/*_rsa.pub".to_string(),
        "**/*.key".to_string(),
        "**/*.pem".to_string(),
        "**/credentials".to_string(),
        "**/.ssh/**".to_string(),
    ]
}

impl Config {
    /// Load config from default location or create default if not found
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config file
            let config = Config::default();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(&config)?;
            std::fs::write(&path, content)?;
            Ok(config)
        }
    }

    /// Get config file path
    pub fn config_path() -> PathBuf {
        if let Some(dirs) = directories::ProjectDirs::from("com", "ctx", "ctx") {
            dirs.config_dir().join("config.toml")
        } else {
            PathBuf::from("~/.ctx/config.toml")
        }
    }
}

// ============================================================================
// Project Config (ctx.toml)
// ============================================================================

/// Project-level configuration (ctx.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    /// Project-level settings
    #[serde(default)]
    pub config: ProjectSettings,

    /// Pack definitions
    #[serde(default)]
    pub packs: HashMap<String, PackDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Default token budget for packs in this project
    #[serde(default = "default_budget")]
    pub default_budget: usize,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            default_budget: default_budget(),
        }
    }
}

/// Pack definition in ctx.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackDefinition {
    /// Token budget (optional, uses project default)
    pub budget: Option<usize>,

    /// Artifacts in this pack
    #[serde(default)]
    pub artifacts: Vec<ArtifactDefinition>,
}

/// Artifact definition in ctx.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDefinition {
    /// Source URI (file:path, glob:pattern, text:content, git:diff)
    pub source: String,

    /// Priority (higher = included first)
    #[serde(default)]
    pub priority: i64,
}

impl ProjectConfig {
    /// Find and load ctx.toml from current or parent directories
    pub fn find_and_load() -> anyhow::Result<Option<(PathBuf, Self)>> {
        if let Some(path) = Self::find_project_root()? {
            let config = Self::load(&path)?;
            Ok(Some((path, config)))
        } else {
            Ok(None)
        }
    }

    /// Find ctx.toml by walking up from current directory
    pub fn find_project_root() -> anyhow::Result<Option<PathBuf>> {
        let current = std::env::current_dir()?;
        Self::find_project_root_from(&current)
    }

    /// Find ctx.toml by walking up from given directory
    pub fn find_project_root_from(start: &Path) -> anyhow::Result<Option<PathBuf>> {
        let mut current = start.to_path_buf();

        loop {
            let ctx_toml = current.join("ctx.toml");
            if ctx_toml.exists() {
                return Ok(Some(current));
            }

            if !current.pop() {
                break;
            }
        }

        Ok(None)
    }

    /// Load ctx.toml from project root
    pub fn load(project_root: &Path) -> anyhow::Result<Self> {
        let path = project_root.join("ctx.toml");
        let content = std::fs::read_to_string(&path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save ctx.toml to project root
    pub fn save(&self, project_root: &Path) -> anyhow::Result<()> {
        let path = project_root.join("ctx.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get project namespace from directory name
    pub fn project_namespace(project_root: &Path) -> String {
        project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Create namespaced pack name
    pub fn namespaced_pack_name(project_root: &Path, pack_name: &str) -> String {
        format!("{}:{}", Self::project_namespace(project_root), pack_name)
    }

    /// Strip namespace from pack name if it matches this project
    pub fn strip_namespace(project_root: &Path, full_name: &str) -> Option<String> {
        let prefix = format!("{}:", Self::project_namespace(project_root));
        full_name.strip_prefix(&prefix).map(String::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.budget_tokens, 128_000);
        assert_eq!(config.mcp.port, 17373);
        assert!(!config.denylist.patterns.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.budget_tokens, config.budget_tokens);
    }

    #[test]
    fn test_denylist_patterns() {
        let config = Config::default();
        assert!(config.denylist.patterns.contains(&"**/.env*".to_string()));
        assert!(config.denylist.patterns.contains(&"**/.aws/**".to_string()));
    }

    #[test]
    fn test_project_config_parse() {
        let toml_str = r#"
[config]
default_budget = 50000

[packs.style]
budget = 128000
artifacts = [
    { source = "file:CONTRIBUTING.md", priority = 10 },
    { source = "glob:src/**/*.rs" },
]

[packs.architecture]
artifacts = [
    { source = "file:README.md" },
]
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.config.default_budget, 50000);
        assert_eq!(config.packs.len(), 2);
        assert!(config.packs.contains_key("style"));
        assert!(config.packs.contains_key("architecture"));

        let style = &config.packs["style"];
        assert_eq!(style.budget, Some(128000));
        assert_eq!(style.artifacts.len(), 2);
        assert_eq!(style.artifacts[0].source, "file:CONTRIBUTING.md");
        assert_eq!(style.artifacts[0].priority, 10);
    }

    #[test]
    fn test_namespaced_pack_name() {
        let root = PathBuf::from("/home/user/my-project");
        assert_eq!(
            ProjectConfig::namespaced_pack_name(&root, "style"),
            "my-project:style"
        );
    }

    #[test]
    fn test_strip_namespace() {
        let root = PathBuf::from("/home/user/my-project");
        assert_eq!(
            ProjectConfig::strip_namespace(&root, "my-project:style"),
            Some("style".to_string())
        );
        assert_eq!(
            ProjectConfig::strip_namespace(&root, "other-project:style"),
            None
        );
    }
}
