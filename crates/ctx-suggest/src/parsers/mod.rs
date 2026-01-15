//! Import parsers for different languages

pub mod python;
pub mod rust;
pub mod typescript;

use std::path::Path;

use anyhow::Result;

/// Parse imports from a file based on its extension
pub async fn parse_imports(path: &Path) -> Result<Vec<String>> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "rs" => rust::parse_imports(path).await,
        "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" => typescript::parse_imports(path).await,
        "py" => python::parse_imports(path).await,
        _ => Ok(vec![]),
    }
}

/// Check if a file extension is supported for import parsing
pub fn is_supported_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" | "py"
    )
}
