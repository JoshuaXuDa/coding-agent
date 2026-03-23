//! Edit tool - Application layer
//!
//! Orchestrates file system operations to provide string replacement functionality.

use anyhow::Result;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::{
    validation::{validate_path, validate_non_empty_string},
    xml_builder::XmlBuilder,
    error_handler::ErrorHandler,
    file_operations::FileOperationPrechecker,
    permissions::PermissionChecker,
    concurrency::FileLockManager,
};

/// Edit tool
///
/// Provides string replacement capabilities in files.
pub struct EditTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
    /// File operation prechecker
    prechecker: FileOperationPrechecker,
    /// Permission checker
    permission_checker: PermissionChecker,
    /// File lock manager
    lock_manager: Arc<FileLockManager>,
}

impl EditTool {
    /// Create a new edit tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        let prechecker = FileOperationPrechecker::new(fs.clone());
        let permission_checker = PermissionChecker::new(fs.clone());
        let lock_manager = Arc::new(FileLockManager::new());

        Self {
            fs,
            prechecker,
            permission_checker,
            lock_manager,
        }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<EditArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        // Validate path
        validate_path(path)?;

        let old_str = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_string' argument"))?;

        // Validate old_string is not empty
        validate_non_empty_string(old_str, "old_string")?;

        let new_str = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_string' argument"))?;

        let replace_all = args
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(EditArgs {
            path: path.to_string(),
            old_string: old_str.to_string(),
            new_string: new_str.to_string(),
            replace_all,
        })
    }
}

/// Edit tool arguments
#[derive(Debug, Clone)]
struct EditArgs {
    path: String,
    old_string: String,
    new_string: String,
    replace_all: bool,
}

impl Tool for EditTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "edit".to_string(),
            name: "edit".to_string(),
            description: "Replace text in a file with new text".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "Text to replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "New text to replace with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false, only first)",
                    "default": false
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
            let edit_args = Self::parse_args(&args)
                .map_err(ErrorHandler::to_tool_error)?;

            let path = Path::new(&edit_args.path);

            // Check if file exists and is writable using prechecker
            let file_info = self.prechecker.verify_file_writable(path).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            // Check write permissions
            let perm_status = self.permission_checker.check_write_permission(path).await
                .map_err(ErrorHandler::to_tool_error)?;

            if !perm_status.allowed {
                let xml = XmlBuilder::build_error(
                    "edit",
                    "PERMISSION_DENIED",
                    &perm_status.reason.unwrap_or_else(|| "Permission denied".to_string()),
                    &perm_status.suggestion.unwrap_or_else(|| "Check file permissions".to_string()),
                ).map_err(ErrorHandler::to_tool_error)?;

                return Ok(ToolResult::success("edit", xml));
            }

            // Acquire write lock for concurrent access protection
            let _lock = self.lock_manager.acquire_write_lock(path).await;

            // Read file content
            let content = self.fs.read_file(path).await
                .map_err(ErrorHandler::to_tool_error)?;

            // Perform replacement
            let new_content = if edit_args.replace_all {
                content.replace(&edit_args.old_string, &edit_args.new_string)
            } else {
                content.replacen(&edit_args.old_string, &edit_args.new_string, 1)
            };

            // Check if replacement was made
            if new_content == content {
                let xml = XmlBuilder::build_error(
                    "edit",
                    "STRING_NOT_FOUND",
                    &format!("String not found: {}", edit_args.old_string),
                    &format!("The old string '{}' was not found in the file", edit_args.old_string),
                ).map_err(ErrorHandler::to_tool_error)?;

                return Ok(ToolResult::success("edit", xml));
            }

            // Write modified content back
            self.fs.write_file(path, &new_content).await
                .map_err(ErrorHandler::to_tool_error)?;

            // Build success XML
            let replacement_count = if edit_args.replace_all {
                let count = content.matches(&edit_args.old_string).count();
                count
            } else {
                1
            };

            let content = format!(
                "<file path=\"{}\"><replacements>{}</replacements></file>",
                crate::tools::domain::escape_xml(&edit_args.path),
                replacement_count
            );

            let summary = format!("Successfully replaced {} occurrence(s)", replacement_count);

            let xml = XmlBuilder::build_success("edit", &content, &summary)
                .map_err(ErrorHandler::to_tool_error)?;

            Ok(ToolResult::success("edit", xml))
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
            "old_string": "hello",
            "new_string": "world"
        });
        let parsed = EditTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
        assert_eq!(parsed.old_string, "hello");
        assert_eq!(parsed.new_string, "world");
        assert!(!parsed.replace_all);

        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "old_string": "foo",
            "new_string": "bar",
            "replace_all": true
        });
        let parsed = EditTool::parse_args(&args).unwrap();
        assert!(parsed.replace_all);
    }

    #[test]
    fn test_parse_args_empty_path() {
        let args = serde_json::json!({
            "path": "",
            "old_string": "hello",
            "new_string": "world"
        });
        assert!(EditTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_path_traversal() {
        let args = serde_json::json!({
            "path": "../../etc/passwd",
            "old_string": "hello",
            "new_string": "world"
        });
        assert!(EditTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_empty_old_string() {
        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "old_string": "",
            "new_string": "world"
        });
        assert!(EditTool::parse_args(&args).is_err());

        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "old_string": "   ",
            "new_string": "world"
        });
        assert!(EditTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_valid_empty_new_string() {
        // new_string can be empty (for deletion)
        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "old_string": "hello",
            "new_string": ""
        });
        assert!(EditTool::parse_args(&args).is_ok());
    }
}
