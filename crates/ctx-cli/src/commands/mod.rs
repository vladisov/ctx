pub mod init;
pub mod mcp;
pub mod pack;
pub mod suggest;
pub mod ui;
pub mod web;

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Find workspace root by looking for .git, Cargo.toml, or package.json
pub fn find_workspace_root(file: &Path) -> Result<PathBuf> {
    let mut current = if file.is_file() {
        file.parent().unwrap_or(file).to_owned()
    } else {
        file.to_path_buf()
    };

    loop {
        if current.join(".git").exists()
            || current.join("Cargo.toml").exists()
            || current.join("package.json").exists()
        {
            return Ok(current);
        }

        if !current.pop() {
            return Ok(file.parent().unwrap_or(file).to_owned());
        }
    }
}
