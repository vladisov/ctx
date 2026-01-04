use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}
