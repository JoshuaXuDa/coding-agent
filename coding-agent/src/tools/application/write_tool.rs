//! Write tool - Application layer
//!
//! Orchestrates file system operations to provide file writing functionality.

use anyhow::Result;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::xml_builder::XmlBuilder;

/// Write tool
///
/// Provides file writing capabilities with validation.
pub struct WriteTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
}

impl WriteTool {
    /// Create a new write tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<WriteArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;

        let create_dirs = args
            .get("create_dirs")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(WriteArgs {
            path: path.to_string(),
            content: content.to_string(),
            create_dirs,
        })
    }

    /// Validate path
    fn validate_path(path: &str) -> Result<()> {
        // Check for suspicious paths
        if path.contains("..") {
            anyhow::bail!("Path contains '..' which may lead to directory traversal");
        }

        // Check if path is empty
        if path.is_empty() {
            anyhow::bail!("Path cannot be empty");
        }

        Ok(())
    }
}

/// Write tool arguments
#[derive(Debug, Clone)]
struct WriteArgs {
    path: String,
    content: String,
    create_dirs: bool,
}

impl Tool for WriteTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "write".to_string(),
            name: "write".to_string(),
            description: "Write content to a file, creating it if it doesn't exist".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                },
                "create_dirs": {
                    "type": "boolean",
                    "description": "Create parent directories if they don't exist (default: true)",
                    "default": true
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
            let write_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Validate path
            Self::validate_path(&write_args.path)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            let path = Path::new(&write_args.path);

            // Check if path exists and is a directory
            if self.fs.exists(path) && self.fs.is_dir(path) {
                let xml = XmlBuilder::build_error(
                    "write",
                    "IS_DIRECTORY",
                    &format!("Cannot write to directory: {}", write_args.path),
                    &format!("The path '{}' is a directory", write_args.path),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("write", xml));
            }

            // Write file
            self.fs.write_file(path, &write_args.content).await
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

            // Build XML response
            let bytes_written = write_args.content.len();
            let xml = XmlBuilder::build_write_result_xml(&write_args.path, bytes_written)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("write", xml))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "content": "Hello, World!"
        });
        let parsed = WriteTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
        assert_eq!(parsed.content, "Hello, World!");
        assert!(parsed.create_dirs);
    }

    #[test]
    fn test_validate_path() {
        assert!(WriteTool::validate_path("/tmp/test.txt").is_ok());
        assert!(WriteTool::validate_path("").is_err());
        assert!(WriteTool::validate_path("/tmp/../etc/passwd").is_err());
    }
}
