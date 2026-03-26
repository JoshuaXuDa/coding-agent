//! WebSearch tool - Application layer
//!
//! Performs web search using the Exa API.

use anyhow::Result;
use std::pin::Pin;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::tools::domain::xml_builder::XmlBuilder;

#[cfg(feature = "websearch")]
use reqwest::Client;

/// API configuration
#[cfg(feature = "websearch")]
const API_BASE_URL: &str = "https://mcp.exa.ai";
const API_ENDPOINT: &str = "/mcp";

/// Default number of results
const DEFAULT_NUM_RESULTS: u8 = 8;

/// WebSearch tool
///
/// Performs web search using the Exa API.
#[cfg(feature = "websearch")]
pub struct WebSearchTool {
    /// HTTP client
    client: Client,
}

#[cfg(feature = "websearch")]
impl WebSearchTool {
    /// Create a new web search tool
    pub fn new() -> Self {
        let client = Client::builder()
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<WebSearchArgs> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?
            .to_string();

        let num_results = args
            .get("numResults")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8)
            .unwrap_or(DEFAULT_NUM_RESULTS);

        let livecrawl = args
            .get("livecrawl")
            .and_then(|v| v.as_str())
            .map(String::from);

        let search_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .map(String::from);

        let context_max_characters = args
            .get("contextMaxCharacters")
            .and_then(|v| v.as_u64());

        Ok(WebSearchArgs {
            query,
            num_results,
            livecrawl,
            search_type,
            context_max_characters,
        })
    }
}

#[cfg(feature = "websearch")]
#[derive(Debug, Clone)]
struct WebSearchArgs {
    query: String,
    num_results: u8,
    livecrawl: Option<String>,
    search_type: Option<String>,
    context_max_characters: Option<u64>,
}

#[cfg(feature = "websearch")]
impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "websearch")]
impl Tool for WebSearchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "websearch".to_string(),
            name: "websearch".to_string(),
            description: "Perform web search using the Exa API".to_string(),
            category: Some("network".to_string()),
            parameters: serde_json::json!({
                "query": {
                    "type": "string",
                    "description": "Web search query"
                },
                "numResults": {
                    "type": "number",
                    "description": "Number of search results to return (default: 8)",
                    "default": 8
                },
                "livecrawl": {
                    "type": "string",
                    "enum": ["fallback", "preferred"],
                    "description": "Live crawl mode - 'fallback': use live crawling as backup, 'preferred': prioritize live crawling"
                },
                "type": {
                    "type": "string",
                    "enum": ["auto", "fast", "deep"],
                    "description": "Search type - 'auto': balanced, 'fast': quick results, 'deep': comprehensive"
                },
                "contextMaxCharacters": {
                    "type": "number",
                    "description": "Maximum characters for context string (default: 10000)"
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
            let search_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Build request
            let request_body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "web_search_exa",
                    "arguments": {
                        "query": search_args.query,
                        "numResults": search_args.num_results,
                        "livecrawl": search_args.livecrawl.unwrap_or_else(|| "fallback".to_string()),
                        "type": search_args.search_type.unwrap_or_else(|| "auto".to_string()),
                        "contextMaxCharacters": search_args.context_max_characters
                    }
                }
            });

            // Send request
            let response = self.client
                .post(&format!("{}{}", API_BASE_URL, API_ENDPOINT))
                .header("accept", "application/json, text/event-stream")
                .header("content-type", "application/json")
                .json(&request_body)
                .timeout(std::time::Duration::from_secs(25))
                .send()
                .await
                .map_err(|e: reqwest::Error| ToolError::ExecutionFailed(format!("Search request failed: {}", e)))?;

            // Check response status
            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(ToolError::ExecutionFailed(format!(
                    "Search error ({}): {}",
                    status,
                    error_text
                )));
            }

            // Parse SSE response
            let response_text = response.text().await
                .map_err(|e: reqwest::Error| ToolError::ExecutionFailed(format!("Failed to read response: {}", e)))?;

            // Parse SSE format
            let mut output = String::new();
            for line in response_text.lines() {
                if line.starts_with("data: ") {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&line[6..]) {
                        if let Some(result) = data.get("result")
                            .and_then(|r| r.get("content"))
                            .and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|item| item.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            output = result.to_string();
                            break;
                        }
                    }
                }
            }

            if output.is_empty() {
                output = "No search results found. Please try a different query.".to_string();
            }

            // Build XML response
            let title = format!("Web search: {}", search_args.query);
            let xml = XmlBuilder::build_success(
                "websearch",
                &title,
                &output,
            ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("websearch", xml))
        })
    }
}

// Stub implementation when websearch feature is not enabled
#[cfg(not(feature = "websearch"))]
pub struct WebSearchTool;

#[cfg(not(feature = "websearch"))]
impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "websearch"))]
impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "websearch"))]
impl Tool for WebSearchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "websearch".to_string(),
            name: "websearch".to_string(),
            description: "Web search (requires 'websearch' feature)".to_string(),
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
                "WebSearch tool requires the 'websearch' feature to be enabled. Run with: cargo build --features websearch".to_string()
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
            "query": "Rust programming"
        });

        #[cfg(feature = "websearch")]
        let result = WebSearchTool::parse_args(&args).unwrap();
        #[cfg(feature = "websearch")]
        assert_eq!(result.query, "Rust programming");
    }
}
