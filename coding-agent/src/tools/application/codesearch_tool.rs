//! CodeSearch tool - Application layer
//!
//! Performs code search using the Exa API.

use anyhow::Result;
use std::pin::Pin;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::tools::domain::json_builder::JsonBuilder;
use serde_json::json;

#[cfg(feature = "codesearch")]
use reqwest::Client;

/// API configuration
#[cfg(feature = "codesearch")]
const API_BASE_URL: &str = "https://mcp.exa.ai";
const API_ENDPOINT: &str = "/mcp";

/// Default number of tokens
const DEFAULT_TOKENS_NUM: u32 = 5000;

/// Minimum tokens
const MIN_TOKENS_NUM: u32 = 1000;

/// Maximum tokens
const MAX_TOKENS_NUM: u32 = 50000;

/// CodeSearch tool
///
/// Performs code search using the Exa API.
#[cfg(feature = "codesearch")]
pub struct CodeSearchTool {
    /// HTTP client
    client: Client,
}

#[cfg(feature = "codesearch")]
impl CodeSearchTool {
    /// Create a new code search tool
    pub fn new() -> Self {
        let client = Client::builder()
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<CodeSearchArgs> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?
            .to_string();

        let tokens_num = args
            .get("tokensNum")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_TOKENS_NUM);

        if tokens_num < MIN_TOKENS_NUM || tokens_num > MAX_TOKENS_NUM {
            return Err(anyhow::anyhow!(
                "tokensNum must be between {} and {}",
                MIN_TOKENS_NUM,
                MAX_TOKENS_NUM
            ));
        }

        Ok(CodeSearchArgs {
            query,
            tokens_num,
        })
    }
}

#[cfg(feature = "codesearch")]
#[derive(Debug, Clone)]
struct CodeSearchArgs {
    query: String,
    tokens_num: u32,
}

#[cfg(feature = "codesearch")]
impl Default for CodeSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "codesearch")]
impl Tool for CodeSearchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "codesearch".to_string(),
            name: "codesearch".to_string(),
            description: "Search for code examples and documentation using the Exa API".to_string(),
            category: Some("network".to_string()),
            parameters: serde_json::json!({
                "query": {
                    "type": "string",
                    "description": "Search query to find relevant context for APIs, Libraries, and SDKs"
                },
                "tokensNum": {
                    "type": "number",
                    "description": "Number of tokens to return (1000-50000). Default is 5000.",
                    "default": 5000,
                    "minimum": 1000,
                    "maximum": 50000
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
                    "name": "get_code_context_exa",
                    "arguments": {
                        "query": search_args.query,
                        "tokensNum": search_args.tokens_num
                    }
                }
            });

            // Send request
            let response = self.client
                .post(&format!("{}{}", API_BASE_URL, API_ENDPOINT))
                .header("accept", "application/json, text/event-stream")
                .header("content-type", "application/json")
                .json(&request_body)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
                .map_err(|e: reqwest::Error| ToolError::ExecutionFailed(format!("Code search request failed: {}", e)))?;

            // Check response status
            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(ToolError::ExecutionFailed(format!(
                    "Code search error ({}): {}",
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
                output = "No code snippets or documentation found. Please try a different query or be more specific about the library or programming concept.".to_string();
            }

            let data = json!({
                "query": &search_args.query,
                "results": &output
            });
            let result = JsonBuilder::build_success("codesearch", data);
            Ok(ToolResult::success("codesearch", result))
        })
    }
}

// Stub implementation when codesearch feature is not enabled
#[cfg(not(feature = "codesearch"))]
pub struct CodeSearchTool;

#[cfg(not(feature = "codesearch"))]
impl CodeSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "codesearch"))]
impl Default for CodeSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "codesearch"))]
impl Tool for CodeSearchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "codesearch".to_string(),
            name: "codesearch".to_string(),
            description: "Code search (requires 'codesearch' feature)".to_string(),
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
                "CodeSearch tool requires the 'codesearch' feature to be enabled. Run with: cargo build --features codesearch".to_string()
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
            "query": "React useState"
        });

        #[cfg(feature = "codesearch")]
        let result = CodeSearchTool::parse_args(&args).unwrap();
        #[cfg(feature = "codesearch")]
        assert_eq!(result.query, "React useState");
    }

    #[test]
    fn test_parse_args_invalid_tokens() {
        let args = serde_json::json!({
            "query": "React useState",
            "tokensNum": 500  // Too low
        });

        #[cfg(feature = "codesearch")]
        let result = CodeSearchTool::parse_args(&args);
        #[cfg(feature = "codesearch")]
        assert!(result.is_err());
    }
}
