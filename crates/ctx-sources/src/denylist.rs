use glob::Pattern;

/// Simple denylist checker using glob patterns
pub struct Denylist {
    patterns: Vec<Pattern>,
}

impl Denylist {
    /// Create new denylist from pattern strings
    pub fn new(patterns: Vec<String>) -> Self {
        let compiled: Vec<Pattern> = patterns
            .into_iter()
            .filter_map(|p| Pattern::new(&p).ok())
            .collect();

        Self { patterns: compiled }
    }

    /// Check if a path matches any deny pattern
    pub fn is_denied(&self, path: &str) -> bool {
        self.patterns.iter().any(|pattern| pattern.matches(path))
    }

    /// Get first matching pattern (for error messages)
    pub fn matching_pattern(&self, path: &str) -> Option<String> {
        self.patterns
            .iter()
            .find(|p| p.matches(path))
            .map(|p| p.as_str().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_deny() {
        let denylist = Denylist::new(vec!["**/.env*".to_string(), "**/*.key".to_string()]);

        assert!(denylist.is_denied(".env"));
        assert!(denylist.is_denied("config/.env"));
        assert!(denylist.is_denied("secrets/api.key"));
        assert!(!denylist.is_denied("README.md"));
    }

    #[test]
    fn test_directory_patterns() {
        let denylist = Denylist::new(vec!["**/.aws/**".to_string(), "**/secrets/**".to_string()]);

        assert!(denylist.is_denied(".aws/credentials"));
        assert!(denylist.is_denied("home/user/.aws/config"));
        assert!(denylist.is_denied("secrets/api_key.txt"));
        assert!(!denylist.is_denied("aws_config.toml"));
    }

    #[test]
    fn test_matching_pattern() {
        let denylist = Denylist::new(vec!["**/.env*".to_string()]);

        let pattern = denylist.matching_pattern(".env");
        assert_eq!(pattern, Some("**/.env*".to_string()));

        let no_match = denylist.matching_pattern("README.md");
        assert_eq!(no_match, None);
    }
}
