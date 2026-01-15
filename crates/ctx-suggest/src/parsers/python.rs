//! Python import parser

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

// Matches: import foo, bar, baz
static IMPORT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*import\s+([\w.,\s]+)").unwrap());

// Matches: from foo.bar import baz
static FROM_IMPORT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*from\s+([\w.]+)\s+import").unwrap());

// Matches: from . import foo (relative import)
static FROM_RELATIVE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*from\s+(\.+[\w.]*)\s+import").unwrap());

/// Parse imports from a Python file
pub async fn parse_imports(path: &Path) -> Result<Vec<String>> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut imports = Vec::new();

    for line in content.lines() {
        // Check for relative imports first
        if let Some(cap) = FROM_RELATIVE_REGEX.captures(line) {
            imports.push(cap[1].to_string());
            continue;
        }

        // Check for from X import
        if let Some(cap) = FROM_IMPORT_REGEX.captures(line) {
            imports.push(cap[1].to_string());
            continue;
        }

        // Check for import X
        if let Some(cap) = IMPORT_REGEX.captures(line) {
            for module in cap[1].split(',') {
                let module = module.trim();
                // Handle "import foo as bar" -> just "foo"
                let module = module.split_whitespace().next().unwrap_or(module);
                if !module.is_empty() {
                    imports.push(module.to_string());
                }
            }
        }
    }

    // Deduplicate
    imports.sort();
    imports.dedup();

    Ok(imports)
}

/// Resolve a Python import to a file path
pub fn resolve_import(
    workspace: &Path,
    source_file: &Path,
    import: &str,
) -> Option<std::path::PathBuf> {
    let source_dir = source_file.parent()?;

    // Handle relative imports (starting with .)
    if import.starts_with('.') {
        return resolve_relative_import(source_dir, import);
    }

    // Try to find as a local module
    let parts: Vec<&str> = import.split('.').collect();

    // Try from workspace root
    if let Some(path) = resolve_module_path(workspace, &parts) {
        return Some(path);
    }

    // Try from source directory
    if let Some(path) = resolve_module_path(source_dir, &parts) {
        return Some(path);
    }

    None
}

fn resolve_relative_import(source_dir: &Path, import: &str) -> Option<std::path::PathBuf> {
    let dots = import.chars().take_while(|&c| c == '.').count();
    let module_part = &import[dots..];

    // Go up (dots - 1) directories (. = current, .. = parent, etc.)
    let mut base = source_dir.to_owned();
    for _ in 1..dots {
        base = base.parent()?.to_owned();
    }

    if module_part.is_empty() {
        // from . import X -> look for __init__.py
        let init = base.join("__init__.py");
        if init.exists() {
            return Some(init);
        }
        return None;
    }

    let parts: Vec<&str> = module_part.split('.').collect();
    resolve_module_path(&base, &parts)
}

fn resolve_module_path(base: &Path, parts: &[&str]) -> Option<std::path::PathBuf> {
    if parts.is_empty() {
        return None;
    }

    let mut current = base.to_owned();

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;

        if is_last {
            // Try module.py
            let file_path = current.join(format!("{part}.py"));
            if file_path.exists() {
                return Some(file_path);
            }

            // Try module/__init__.py
            let init_path = current.join(part).join("__init__.py");
            if init_path.exists() {
                return Some(init_path);
            }

            return None;
        }
        // Navigate into package directory
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
    fn test_import_regex() {
        let line = "import os, sys, json";
        let cap = IMPORT_REGEX.captures(line).unwrap();
        assert_eq!(&cap[1], "os, sys, json");
    }

    #[test]
    fn test_from_import_regex() {
        let line = "from pathlib import Path";
        let cap = FROM_IMPORT_REGEX.captures(line).unwrap();
        assert_eq!(&cap[1], "pathlib");
    }

    #[test]
    fn test_relative_import() {
        let line = "from ..utils import helper";
        let cap = FROM_RELATIVE_REGEX.captures(line).unwrap();
        assert_eq!(&cap[1], "..utils");
    }
}
