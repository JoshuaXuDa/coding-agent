//! List tool - Application layer
//!
//! Orchestrates file system operations to provide directory listing functionality.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::xml_builder::XmlBuilder;

/// List tool
///
/// Provides directory listing capabilities with file metadata.
pub struct ListTool {
    /// File system service
    fs: Arc<dyn FileSystem>,

    /// XML builder service
    xml_builder: XmlBuilder,
}

impl ListTool {
    /// Create a new list tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self {
            fs,
            xml_builder: XmlBuilder::new(),
        }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<ListArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let show_hidden = args
            .get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(ListArgs {
            path: path.to_string(),
            show_hidden,
        })
    }
}

/// List tool arguments
#[derive(Debug, Clone)]
struct ListArgs {
    path: String,
    show_hidden: bool,
}

impl Tool for ListTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "list".to_string(),
            name: "list".to_string(),
            description: "List directory contents with detailed file metadata".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default: current directory)",
                    "default": "."
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Show hidden files and directories (default: false)",
                    "default": false
                }
            }),
            metadata: Default::default(), // Use default for now
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        args: serde_json::Value,
        _context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            // Parse arguments
            let list_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Check if path exists
            let path = Path::new(&list_args.path);
            if !self.fs.exists(path) {
                let xml = XmlBuilder::build_error(
                    "list",
                    "PATH_NOT_FOUND",
                    &format!("Path not found: {}", list_args.path),
                    &format!("The path '{}' does not exist", list_args.path),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("list", xml));
            }

            // Check if path is a directory
            if !self.fs.is_dir(path) {
                let xml = XmlBuilder::build_error(
                    "list",
                    "NOT_A_DIRECTORY",
                    &format!("Not a directory: {}", list_args.path),
                    &format!("The path '{}' is not a directory", list_args.path),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("list", xml));
            }

            // List directory
            let mut entries = self.fs.list_dir(path).await
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Filter hidden files if requested
            if !list_args.show_hidden {
                entries.retain(|e| !e.is_hidden);
            }

            // Build XML response
            let xml = XmlBuilder::build_directory_xml(&list_args.path, entries)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("list", xml))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::create_filesystem;

    #[tokio::test]
    async fn test_parse_args() {
        let args = serde_json::json!({});
        let parsed = ListTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, ".");
        assert!(!parsed.show_hidden);

        let args = serde_json::json!({"path": "/tmp", "show_hidden": true});
        let parsed = ListTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, "/tmp");
        assert!(parsed.show_hidden);
    }

    #[tokio::test]
    async fn test_list_current_dir() {
        let fs = create_filesystem();
        let tool = ListTool::new(fs);

        // Create a minimal context
        let context = ToolCallContext {
            agent_id: "test",
            run_id: "test",
            thread_id: "test",
            tool_call_id: "test",
            state: None,
        };

        let result = tool.execute(serde_json::json!({}), &context).await.unwrap();
        assert!(result.content.contains("<directory"));
        assert!(result.content.contains("<entry>"));
    }
}
