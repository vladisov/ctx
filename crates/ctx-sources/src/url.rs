use async_trait::async_trait;
use ctx_core::{Artifact, ArtifactType, Error, Result};
use regex::Regex;

use crate::handler::{SourceHandler, SourceOptions};

pub struct UrlHandler;

impl UrlHandler {
    /// Convert HTML to plain text by stripping tags and decoding entities
    fn html_to_text(html: &str) -> String {
        // Remove script and style tags with their content
        let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
        let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
        let text = script_re.replace_all(html, "");
        let text = style_re.replace_all(&text, "");

        // Remove all other HTML tags
        let tag_re = Regex::new(r"<[^>]+>").unwrap();
        let text = tag_re.replace_all(&text, " ");

        // Decode common HTML entities
        let text = text
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'");

        // Collapse multiple whitespace into single space
        let ws_re = Regex::new(r"\s+").unwrap();
        let text = ws_re.replace_all(&text, " ");

        text.trim().to_string()
    }

    /// Extract title from HTML
    fn extract_title(html: &str) -> Option<String> {
        let title_re = Regex::new(r"(?is)<title[^>]*>([^<]+)</title>").ok()?;
        title_re
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    }
}

#[async_trait]
impl SourceHandler for UrlHandler {
    async fn parse(&self, uri: &str, _options: SourceOptions) -> Result<Artifact> {
        let url = if let Some(url) = uri.strip_prefix("url:") {
            url.to_string()
        } else {
            return Err(Error::InvalidSourceUri(format!("Invalid URL URI: {}", uri)));
        };

        // Validate URL format
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(Error::InvalidSourceUri(format!(
                "URL must start with http:// or https://: {}",
                url
            )));
        }

        // Create artifact with URL type (content fetched on load)
        Ok(Artifact::new(
            ArtifactType::Url { url, title: None },
            uri.to_string(),
        ))
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        match &artifact.artifact_type {
            ArtifactType::Url { url, .. } => {
                let client = reqwest::Client::builder()
                    .user_agent("ctx/1.0 (context aggregator)")
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .map_err(|e| {
                        Error::Other(anyhow::anyhow!("Failed to create HTTP client: {}", e))
                    })?;

                let response = client
                    .get(url)
                    .send()
                    .await
                    .map_err(|e| Error::Other(anyhow::anyhow!("Failed to fetch URL: {}", e)))?;

                if !response.status().is_success() {
                    return Err(Error::Other(anyhow::anyhow!(
                        "HTTP error {}: {}",
                        response.status().as_u16(),
                        url
                    )));
                }

                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                let body = response
                    .text()
                    .await
                    .map_err(|e| Error::Other(anyhow::anyhow!("Failed to read response: {}", e)))?;

                // If HTML, convert to text
                let text = if content_type.contains("text/html") {
                    let title = Self::extract_title(&body);
                    let text = Self::html_to_text(&body);
                    if let Some(title) = title {
                        format!("# {}\n\n{}", title, text)
                    } else {
                        text
                    }
                } else {
                    body
                };

                Ok(text)
            }
            _ => Err(Error::Other(anyhow::anyhow!(
                "Unsupported artifact type for UrlHandler"
            ))),
        }
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("url:")
    }
}
