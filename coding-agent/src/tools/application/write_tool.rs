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
use crate::tools::domain::{
    validation::validate_content_size,
    xml_builder::XmlBuilder,
    error_handler::ErrorHandler,
    file_operations::FileOperationPrechecker,
    permissions::PermissionChecker,
    concurrency::FileLockManager,
};

/// Write tool
///
/// Provides file writing capabilities with validation.
pub struct WriteTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
    /// File operation prechecker
    prechecker: FileOperationPrechecker,
    /// Permission checker
    permission_checker: PermissionChecker,
    /// File lock manager
    lock_manager: Arc<FileLockManager>,
}

impl WriteTool {
    /// Create a new write tool
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
    fn parse_args(args: &serde_json::Value) -> Result<WriteArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;

        // Validate content size
        validate_content_size(content)?;

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
                .map_err(ErrorHandler::to_tool_error)?;

            let path = Path::new(&write_args.path);

            // Check if we can create/write to the file
            self.prechecker.verify_can_create_file(path).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            // Check write permissions
            let perm_status = self.permission_checker.check_write_permission(path).await
                .map_err(ErrorHandler::to_tool_error)?;

            if !perm_status.allowed {
                let xml = XmlBuilder::build_error(
                    "write",
                    "PERMISSION_DENIED",
                    &perm_status.reason.unwrap_or_else(|| "Permission denied".to_string()),
                    &perm_status.suggestion.unwrap_or_else(|| "Check file permissions".to_string()),
                ).map_err(ErrorHandler::to_tool_error)?;

                return Ok(ToolResult::success("write", xml));
            }

            // Check if path is a directory
            if self.fs.exists(path) && self.fs.is_dir(path) {
                let xml = XmlBuilder::build_error(
                    "write",
                    "IS_DIRECTORY",
                    &format!("Cannot write to directory: {}", write_args.path),
                    &format!("The path '{}' is a directory", write_args.path),
                ).map_err(ErrorHandler::to_tool_error)?;

                return Ok(ToolResult::success("write", xml));
            }

            // Check parent directory write permission if creating new file
            if !self.fs.exists(path) {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() && self.fs.exists(parent) {
                        let dir_perm_status = self.permission_checker.check_directory_create_permission(parent).await
                            .map_err(ErrorHandler::to_tool_error)?;

                        if !dir_perm_status.allowed {
                            let xml = XmlBuilder::build_error(
                                "write",
                                "PERMISSION_DENIED",
                                &dir_perm_status.reason.unwrap_or_else(|| "Cannot create file".to_string()),
                                &dir_perm_status.suggestion.unwrap_or_else(|| "Check parent directory permissions".to_string()),
                            ).map_err(ErrorHandler::to_tool_error)?;

                            return Ok(ToolResult::success("write", xml));
                        }
                    }
                }
            }

            // Acquire write lock for concurrent access protection
            let _lock = self.lock_manager.acquire_write_lock(path).await;

            // Write file
            self.fs.write_file(path, &write_args.content).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

            // Build XML response
            let bytes_written = write_args.content.len();
            let xml = XmlBuilder::build_write_result_xml(&write_args.path, bytes_written)
                .map_err(ErrorHandler::to_tool_error)?;

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
    fn test_parse_args_empty_content() {
        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "content": ""
        });
        assert!(WriteTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_large_content() {
        use crate::tools::domain::validation::MAX_CONTENT_SIZE;

        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "content": "x".repeat(MAX_CONTENT_SIZE + 1)
        });
        assert!(WriteTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_valid_path_traversal() {
        let args = serde_json::json!({
            "path": "../../etc/passwd",
            "content": "malicious"
        });
        // Path validation happens in execute, not parse_args
        assert!(WriteTool::parse_args(&args).is_ok());
    }
}
