//! WebFetch tool - Application layer
//!
//! Fetches content from URLs with format conversion support.

use anyhow::Result;
use std::pin::Pin;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::tools::domain::xml_builder::XmlBuilder;

#[cfg(feature = "web-tools")]
use reqwest::Client;

#[cfg(feature = "web-tools")]
use html2md::parse_html;

/// Maximum response size (5MB)
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;

/// Default timeout (30 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum timeout (120 seconds)
const MAX_TIMEOUT_SECS: u64 = 120;

/// WebFetch tool
///
/// Fetches content from URLs with format conversion support.
#[cfg(feature = "web-tools")]
pub struct WebFetchTool {
    /// HTTP client
    client: Client,
}

#[cfg(feature = "web-tools")]
impl WebFetchTool {
    /// Create a new web fetch tool
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<WebFetchArgs> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?
            .to_string();

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow::anyhow!("URL must start with http:// or https://"));
        }

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown");

        let format = match format {
            "text" | "markdown" | "html" => format.to_string(),
            _ => return Err(anyhow::anyhow!("Invalid format '{}'. Must be 'text', 'markdown', or 'html'", format)),
        };

        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let timeout_secs = timeout_secs.min(MAX_TIMEOUT_SECS);

        Ok(WebFetchArgs {
            url,
            format,
            timeout_secs,
        })
    }

    /// Convert HTML to Markdown
    fn convert_html_to_markdown(&self, html: &str) -> String {
        parse_html(html)
    }

    /// Extract text from HTML (removes scripts, styles, etc.)
    fn extract_text_from_html(&self, html: &str) -> String {
        // Simple text extraction - remove HTML tags
        let re = regex::Regex::new(r"<script[^>]*>.*?</script>|<style[^>]*>.*?</style>|<[^>]+>").unwrap();
        let text = re.replace_all(html, " ");
        // Clean up whitespace
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(feature = "web-tools")]
#[derive(Debug, Clone)]
struct WebFetchArgs {
    url: String,
    format: String,
    timeout_secs: u64,
}

#[cfg(feature = "web-tools")]
impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "web-tools")]
impl Tool for WebFetchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "webfetch".to_string(),
            name: "webfetch".to_string(),
            description: "Fetch content from URLs with format conversion support".to_string(),
            category: Some("network".to_string()),
            parameters: serde_json::json!({
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from"
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html"],
                    "description": "The format to return the content in (text, markdown, or html)",
                    "default": "markdown"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in seconds (max 120)",
                    "default": 30
                }
            }),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        args: serde_json::Value,
        _context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            // Parse arguments
            let fetch_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Build request with timeout
            let timeout = std::time::Duration::from_secs(fetch_args.timeout_secs);

            let response = self.client
                .get(&fetch_args.url)
                .timeout(timeout)
                .send()
                .await
                .map_err(|e: reqwest::Error| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;

            // Check response status
            if !response.status().is_success() {
                return Err(ToolError::ExecutionFailed(format!(
                    "Request failed with status code: {}",
                    response.status()
                )));
            }

            // Check content length
            if let Some(content_length) = response.content_length() {
                if content_length as usize > MAX_RESPONSE_SIZE {
                    return Err(ToolError::ExecutionFailed(
                        "Response too large (exceeds 5MB limit)".to_string()
                    ));
                }
            }

            // Get content type
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("text/plain")
                .to_string();

            // Get response bytes
            let bytes = response
                .bytes()
                .await
                .map_err(|e: reqwest::Error| ToolError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

            if bytes.len() > MAX_RESPONSE_SIZE {
                return Err(ToolError::ExecutionFailed(
                    "Response too large (exceeds 5MB limit)".to_string()
                ));
            }

            // Check if response is an image
            let is_image = content_type.starts_with("image/")
                && !content_type.contains("svg")
                && !content_type.contains("vnd.fastbidsheet");

            let output = if is_image {
                // For images, return a success message
                format!("Image fetched successfully ({} bytes, {})", bytes.len(), content_type)
            } else {
                // Convert bytes to string
                let content = String::from_utf8_lossy(&bytes);

                // Handle content based on requested format
                match fetch_args.format.as_str() {
                    "markdown" => {
                        if content_type.contains("html") {
                            self.convert_html_to_markdown(&content)
                        } else {
                            content.to_string()
                        }
                    }
                    "text" => {
                        if content_type.contains("html") {
                            self.extract_text_from_html(&content)
                        } else {
                            content.to_string()
                        }
                    }
                    "html" => content.to_string(),
                    _ => content.to_string(),
                }
            };

            // Build XML response
            let title = format!("{} ({})", fetch_args.url, content_type);
            let xml = XmlBuilder::build_success(
                "webfetch",
                &title,
                &output,
            ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("webfetch", xml))
        })
    }
}

// Stub implementation when web-tools feature is not enabled
#[cfg(not(feature = "web-tools"))]
pub struct WebFetchTool;

#[cfg(not(feature = "web-tools"))]
impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "web-tools"))]
impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "web-tools"))]
impl Tool for WebFetchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "webfetch".to_string(),
            name: "webfetch".to_string(),
            description: "Fetch content from URLs (requires 'web-tools' feature)".to_string(),
            category: Some("network".to_string()),
            parameters: serde_json::json!({}),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        _args: serde_json::Value,
        _context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            Err(ToolError::ExecutionFailed(
                "WebFetch tool requires the 'web-tools' feature to be enabled. Run with: cargo build --features web-tools".to_string()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "url": "https://example.com",
            "format": "markdown"
        });

        #[cfg(feature = "web-tools")]
        let result = WebFetchTool::parse_args(&args).unwrap();
        #[cfg(feature = "web-tools")]
        assert_eq!(result.url, "https://example.com");
        #[cfg(feature = "web-tools")]
        assert_eq!(result.format, "markdown");
    }

    #[test]
    fn test_parse_args_invalid_url() {
        let args = serde_json::json!({
            "url": "not-a-url",
            "format": "markdown"
        });

        #[cfg(feature = "web-tools")]
        let result = WebFetchTool::parse_args(&args);
        #[cfg(feature = "web-tools")]
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_args_invalid_format() {
        let args = serde_json::json!({
            "url": "https://example.com",
            "format": "invalid"
        });

        #[cfg(feature = "web-tools")]
        let result = WebFetchTool::parse_args(&args);
        #[cfg(feature = "web-tools")]
        assert!(result.is_err());
    }
}
