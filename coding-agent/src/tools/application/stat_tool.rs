//! Stat tool - Application layer
//!
//! Orchestrates file system operations to provide file metadata functionality.

use anyhow::Result;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::json_builder::JsonBuilder;

/// Stat tool
///
/// Provides file metadata retrieval capabilities.
pub struct StatTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
}

impl StatTool {
    /// Create a new stat tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<StatArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        Ok(StatArgs {
            path: path.to_string(),
        })
    }
}

/// Stat tool arguments
#[derive(Debug, Clone)]
struct StatArgs {
    path: String,
}

impl Tool for StatTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "stat".to_string(),
            name: "stat".to_string(),
            description: "Get detailed metadata for a file or directory".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory"
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
            let stat_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            let path = Path::new(&stat_args.path);

            // Check if path exists
            if !self.fs.exists(path) {
                let json = JsonBuilder::build_error(
                    "stat",
                    "PATH_NOT_FOUND",
                    &format!("Path not found: {}", stat_args.path),
                    &format!("The path '{}' does not exist", stat_args.path),
                );

                return Ok(ToolResult::success("stat", json));
            }

            // Get file metadata
            let metadata = self.fs.file_metadata(path).await
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Build JSON response
            let json = JsonBuilder::build_stat_result(&metadata);

            Ok(ToolResult::success("stat", json))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let parsed = StatTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
    }
}
