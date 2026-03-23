//! Read tool - Application layer
//!
//! Orchestrates file system operations to provide file reading functionality.

use anyhow::Result;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::xml_builder::XmlBuilder;
use crate::tools::truncate_output;

/// Read tool
///
/// Provides file reading capabilities with range support.
pub struct ReadTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
}

impl ReadTool {
    /// Create a new read tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<ReadArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let offset = args.get("offset").and_then(|v| v.as_u64()).map(|v| v as usize);
        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        Ok(ReadArgs {
            path: path.to_string(),
            offset,
            limit,
        })
    }

    /// Read file with range support
    async fn read_file_with_range(&self, path: &Path, offset: Option<usize>, limit: Option<usize>) -> Result<String> {
        let content = self.fs.read_file(path).await?;

        let result = match (offset, limit) {
            (Some(offset), Some(limit)) => {
                // Read from offset with limit
                let lines: Vec<&str> = content.lines().skip(offset).take(limit).collect();
                lines.join("\n")
            }
            (Some(offset), None) => {
                // Read from offset to end
                let lines: Vec<&str> = content.lines().skip(offset).collect();
                lines.join("\n")
            }
            (None, Some(limit)) => {
                // Read from start with limit
                let lines: Vec<&str> = content.lines().take(limit).collect();
                lines.join("\n")
            }
            (None, None) => {
                // Read entire file
                content
            }
        };

        Ok(result)
    }
}

/// Read tool arguments
#[derive(Debug, Clone)]
struct ReadArgs {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl Tool for ReadTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "read".to_string(),
            name: "read".to_string(),
            description: "Read file contents with optional offset and limit for partial reading".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "offset": {
                    "type": "number",
                    "description": "Skip this many lines from the start (default: 0)",
                    "default": 0
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of lines to read (default: read all)"
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
            let read_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            let path = Path::new(&read_args.path);

            // Check if path exists
            if !self.fs.exists(path) {
                let xml = XmlBuilder::build_error(
                    "read",
                    "FILE_NOT_FOUND",
                    &format!("File not found: {}", read_args.path),
                    &format!("The file '{}' does not exist", read_args.path),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("read", xml));
            }

            // Check if path is a file
            if !self.fs.is_file(path) {
                let xml = XmlBuilder::build_error(
                    "read",
                    "NOT_A_FILE",
                    &format!("Not a file: {}", read_args.path),
                    &format!("The path '{}' is not a file", read_args.path),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("read", xml));
            }

            // Read file with range
            let content = self.read_file_with_range(path, read_args.offset, read_args.limit).await
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Truncate if too large
            let content = truncate_output(&content);

            // Build XML response
            let xml = XmlBuilder::build_file_content_xml(&read_args.path, &content, read_args.offset, read_args.limit)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("read", xml))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::create_filesystem;

    #[tokio::test]
    async fn test_parse_args() {
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let parsed = ReadTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
        assert!(parsed.offset.is_none());
        assert!(parsed.limit.is_none());

        let args = serde_json::json!({"path": "/tmp/test.txt", "offset": 10, "limit": 20});
        let parsed = ReadTool::parse_args(&args).unwrap();
        assert_eq!(parsed.offset, Some(10));
        assert_eq!(parsed.limit, Some(20));
    }
}
