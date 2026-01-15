//! Rust import parser

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

// Matches: use crate::foo::bar; use super::baz; use self::qux;
static USE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*use\s+((?:crate|super|self)(?:::\w+)+)").unwrap());

// Matches: mod foo; (without body - external module)
static MOD_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*;").unwrap());

/// Parse imports from a Rust file
pub async fn parse_imports(path: &Path) -> Result<Vec<String>> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut imports = Vec::new();

    for line in content.lines() {
        if let Some(cap) = USE_REGEX.captures(line) {
            imports.push(cap[1].to_string());
        }
        if let Some(cap) = MOD_REGEX.captures(line) {
            imports.push(format!("self::{}", &cap[1]));
        }
    }

    Ok(imports)
}

/// Resolve a Rust import to a file path
pub fn resolve_import(
    _workspace: &Path,
    source_file: &Path,
    import: &str,
) -> Option<std::path::PathBuf> {
    let parts: Vec<&str> = import.split("::").collect();
    if parts.is_empty() {
        return None;
    }

    let source_dir = source_file.parent()?;

    match parts[0] {
        "crate" => {
            // Find crate root (look for Cargo.toml)
            let mut crate_root = source_dir.to_owned();
            loop {
                if crate_root.join("Cargo.toml").exists() {
                    break;
                }
                if !crate_root.pop() {
                    return None;
                }
            }

            // Resolve path from crate root/src
            let src_dir = crate_root.join("src");
            resolve_module_path(&src_dir, &parts[1..])
        }
        "super" => {
            // Go up one directory
            let parent = source_dir.parent()?;
            resolve_module_path(parent, &parts[1..])
        }
        "self" => {
            // Same directory
            resolve_module_path(source_dir, &parts[1..])
        }
        _ => None,
    }
}

/// Resolve module path to a file
fn resolve_module_path(base: &Path, parts: &[&str]) -> Option<std::path::PathBuf> {
    if parts.is_empty() {
        return None;
    }

    let mut current = base.to_owned();

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;

        if is_last {
            // Try module.rs
            let file_path = current.join(format!("{part}.rs"));
            if file_path.exists() {
                return Some(file_path);
            }

            // Try module/mod.rs
            let mod_path = current.join(part).join("mod.rs");
            if mod_path.exists() {
                return Some(mod_path);
            }

            return None;
        }
        // Navigate into directory
        current = current.join(part);
        if !current.exists() {
            return None;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_regex() {
        let line = "use crate::foo::bar;";
        let cap = USE_REGEX.captures(line).unwrap();
        assert_eq!(&cap[1], "crate::foo::bar");
    }

    #[test]
    fn test_mod_regex() {
        let line = "pub mod utils;";
        let cap = MOD_REGEX.captures(line).unwrap();
        assert_eq!(&cap[1], "utils");
    }
}
