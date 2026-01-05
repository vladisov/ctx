use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionInfo {
    pub artifact_id: String,
    pub redaction_type: String,
    pub count: usize,
}

/// Simple redaction engine for common secrets
pub struct Redactor {
    patterns: Vec<(String, Regex)>,
}

impl Redactor {
    pub fn new() -> Self {
        let mut patterns = Vec::new();

        // Add common secret patterns (order matters - more specific first)
        patterns.push((
            "AWS_ACCESS_KEY".to_string(),
            Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        ));
        patterns.push((
            "PRIVATE_KEY".to_string(),
            Regex::new(r"-----BEGIN[A-Z ]*PRIVATE KEY-----").unwrap(),
        ));
        patterns.push((
            "GITHUB_TOKEN".to_string(),
            Regex::new(r"gh[ps]_[a-zA-Z0-9]{36,}").unwrap(),
        ));
        patterns.push((
            "JWT".to_string(),
            Regex::new(r"eyJ[a-zA-Z0-9_-]+\.eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+").unwrap(),
        ));
        patterns.push((
            "API_KEY".to_string(),
            Regex::new(r#"(?i)(api[_-]?key|apikey)['"\s:=]+([a-zA-Z0-9_-]{20,})"#).unwrap(),
        ));
        patterns.push((
            "BEARER_TOKEN".to_string(),
            Regex::new(r#"(?i)bearer\s+([a-zA-Z0-9_.\-]{20,})"#).unwrap(),
        ));

        Self { patterns }
    }

    /// Redact secrets from content
    pub fn redact(&self, artifact_id: &str, content: &str) -> (String, Vec<RedactionInfo>) {
        let mut result = content.to_string();
        let mut redactions = Vec::new();

        for (secret_type, pattern) in &self.patterns {
            let matches: Vec<_> = pattern.find_iter(&result).collect();
            let count = matches.len();

            if count > 0 {
                result = pattern
                    .replace_all(&result, format!("[REDACTED:{}]", secret_type))
                    .to_string();

                redactions.push(RedactionInfo {
                    artifact_id: artifact_id.to_string(),
                    redaction_type: secret_type.clone(),
                    count,
                });
            }
        }

        (result, redactions)
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_key_redaction() {
        let redactor = Redactor::new();
        let content = "My AWS key is AKIAIOSFODNN7EXAMPLE";

        let (redacted, info) = redactor.redact("test", content);

        assert!(redacted.contains("[REDACTED:AWS_ACCESS_KEY]"));
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].redaction_type, "AWS_ACCESS_KEY");
        assert_eq!(info[0].count, 1);
    }

    #[test]
    fn test_private_key_redaction() {
        let redactor = Redactor::new();
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA";

        let (redacted, info) = redactor.redact("test", content);

        assert!(redacted.contains("[REDACTED:PRIVATE_KEY]"));
        assert_eq!(info[0].redaction_type, "PRIVATE_KEY");
    }

    #[test]
    fn test_no_secrets() {
        let redactor = Redactor::new();
        let content = "Just some normal code here";

        let (redacted, info) = redactor.redact("test", content);

        assert_eq!(redacted, content);
        assert_eq!(info.len(), 0);
    }
}
