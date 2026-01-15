//! TypeScript/JavaScript import parser

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

// Matches: import ... from './foo' or "../../bar" or '@scope/pkg'
static IMPORT_FROM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#).unwrap());

// Matches: import './styles.css' (side-effect imports)
static IMPORT_SIDE_EFFECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*import\s+['"]([^'"]+)['"]"#).unwrap());

// Matches: require('./foo')
static REQUIRE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap());

// Matches: export ... from './foo'
static EXPORT_FROM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"export\s+.*?\s+from\s+['"]([^'"]+)['"]"#).unwrap());

/// Parse imports from a TypeScript/JavaScript file
pub async fn parse_imports(path: &Path) -> Result<Vec<String>> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut imports = Vec::new();

    for cap in IMPORT_FROM_REGEX.captures_iter(&content) {
        imports.push(cap[1].to_string());
    }
    for cap in IMPORT_SIDE_EFFECT.captures_iter(&content) {
        imports.push(cap[1].to_string());
    }
    for cap in REQUIRE_REGEX.captures_iter(&content) {
        imports.push(cap[1].to_string());
    }
    for cap in EXPORT_FROM_REGEX.captures_iter(&content) {
        imports.push(cap[1].to_string());
    }

    // Deduplicate
    imports.sort();
    imports.dedup();

    Ok(imports)
}

/// Resolve a TypeScript/JavaScript import to a file path
pub fn resolve_import(
    _workspace: &Path,
    source_file: &Path,
    import: &str,
) -> Option<std::path::PathBuf> {
    // Only resolve relative imports
    if !import.starts_with('.') {
        return None;
    }

    let source_dir = source_file.parent()?;
    let import_path = source_dir.join(import);

    // Try various extensions
    let extensions = [
        "",
        ".ts",
        ".tsx",
        ".js",
        ".jsx",
        "/index.ts",
        "/index.tsx",
        "/index.js",
    ];

    for ext in extensions {
        let full_path = if let Some(stripped) = ext.strip_prefix('/') {
            import_path.join(stripped)
        } else {
            std::path::PathBuf::from(format!("{}{}", import_path.display(), ext))
        };

        if full_path.exists() && full_path.is_file() {
            return Some(full_path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_from_regex() {
        let line = r#"import { foo } from './utils';"#;
        let cap = IMPORT_FROM_REGEX.captures(line).unwrap();
        assert_eq!(&cap[1], "./utils");
    }

    #[test]
    fn test_import_side_effect() {
        let line = r#"import './styles.css';"#;
        let cap = IMPORT_SIDE_EFFECT.captures(line).unwrap();
        assert_eq!(&cap[1], "./styles.css");
    }

    #[test]
    fn test_require_regex() {
        let line = r#"const utils = require('./utils');"#;
        let cap = REQUIRE_REGEX.captures(line).unwrap();
        assert_eq!(&cap[1], "./utils");
    }
}
