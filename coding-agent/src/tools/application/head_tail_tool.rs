//! Head/Tail tool - Application layer
//!
//! Orchestrates file system operations to provide partial file reading functionality.

use anyhow::Result;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::json_builder::JsonBuilder;
use crate::tools::truncate_output;

/// Head/Tail tool
///
/// Provides partial file reading capabilities (head or tail).
pub struct HeadTailTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
}

impl HeadTailTool {
    /// Create a new head/tail tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<HeadTailArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let mode = args
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("head");

        let lines = args
            .get("lines")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);

        Ok(HeadTailArgs {
            path: path.to_string(),
            mode: mode.to_string(),
            lines,
        })
    }

    /// Read head of file
    fn read_head(&self, path: &Path, lines: usize) -> Result<String> {
        let rt = tokio::runtime::Runtime::new()?;
        let content = rt.block_on(self.fs.read_file(path))?;

        let result: String = content
            .lines()
            .take(lines)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(result)
    }

    /// Read tail of file
    fn read_tail(&self, path: &Path, lines: usize) -> Result<String> {
        let rt = tokio::runtime::Runtime::new()?;
        let content = rt.block_on(self.fs.read_file(path))?;

        let content_lines: Vec<&str> = content.lines().collect();
        let start = if content_lines.len() > lines {
            content_lines.len() - lines
        } else {
            0
        };

        let result = content_lines[start..].join("\n");

        Ok(result)
    }
}

/// Head/Tail tool arguments
#[derive(Debug, Clone)]
struct HeadTailArgs {
    path: String,
    mode: String,
    lines: usize,
}

impl Tool for HeadTailTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "head_tail".to_string(),
            name: "head_tail".to_string(),
            description: "View first or last N lines of a file".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "mode": {
                    "type": "string",
                    "description": "Reading mode: 'head' for first lines, 'tail' for last lines",
                    "enum": ["head", "tail"],
                    "default": "head"
                },
                "lines": {
                    "type": "number",
                    "description": "Number of lines to read (default: 10)",
                    "default": 10
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
            let ht_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Validate mode
            if ht_args.mode != "head" && ht_args.mode != "tail" {
                let json = JsonBuilder::build_error(
                    "head_tail",
                    "INVALID_MODE",
                    &format!("Invalid mode: {}", ht_args.mode),
                    "Mode must be either 'head' or 'tail'",
                );

                return Ok(ToolResult::success("head_tail", json));
            }

            let path = Path::new(&ht_args.path);

            // Check if path exists
            if !self.fs.exists(path) {
                let json = JsonBuilder::build_error(
                    "head_tail",
                    "FILE_NOT_FOUND",
                    &format!("File not found: {}", ht_args.path),
                    &format!("The file '{}' does not exist", ht_args.path),
                );

                return Ok(ToolResult::success("head_tail", json));
            }

            // Check if path is a file
            if !self.fs.is_file(path) {
                let json = JsonBuilder::build_error(
                    "head_tail",
                    "NOT_A_FILE",
                    &format!("Not a file: {}", ht_args.path),
                    &format!("The path '{}' is not a file", ht_args.path),
                );

                return Ok(ToolResult::success("head_tail", json));
            }

            // Read file content (sync operation)
            let content = if ht_args.mode == "head" {
                self.read_head(path, ht_args.lines)
            } else {
                self.read_tail(path, ht_args.lines)
            }.map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Truncate if too large
            let content = truncate_output(&content);

            // Build JSON response
            let lines_vec: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            let line_count = lines_vec.len();
            let total_lines = line_count;

            let json = JsonBuilder::build_head_tail_result(
                &ht_args.path,
                lines_vec,
                &ht_args.mode,
                line_count,
                total_lines,
            );

            Ok(ToolResult::success("head_tail", json))
        })
    }
}

// --- ToolProvider implementation ---
pub struct HeadTailToolProvider;

impl crate::tools::domain::provider::ToolProvider for HeadTailToolProvider {
    fn tool_id(&self) -> &str { "head_tail" }

    fn dependency_type(&self) -> crate::tools::domain::provider::DependencyType { crate::tools::domain::provider::DependencyType::FileSystem }

    fn build(
        &self,
        fs: Option<Arc<dyn FileSystem>>,
        _executor: Option<Arc<dyn crate::platform::domain::command::CommandExecutor>>,
    ) -> Arc<dyn Tool> {
        Arc::new(HeadTailTool::new(fs.unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let parsed = HeadTailTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
        assert_eq!(parsed.mode, "head");
        assert_eq!(parsed.lines, 10);

        let args = serde_json::json!({
            "path": "/tmp/test.txt",
            "mode": "tail",
            "lines": 20
        });
        let parsed = HeadTailTool::parse_args(&args).unwrap();
        assert_eq!(parsed.mode, "tail");
        assert_eq!(parsed.lines, 20);
    }
}
