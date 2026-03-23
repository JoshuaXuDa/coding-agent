//! Grep tool - Application layer
//!
//! Orchestrates file system operations to provide content search functionality.

use anyhow::Result;
use regex::Regex;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::xml_builder::{XmlBuilder, GrepMatch};

/// Grep tool
///
/// Provides content search capabilities using regex patterns.
pub struct GrepTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
}

impl GrepTool {
    /// Create a new grep tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<GrepArgs> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' argument"))?;

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let case_insensitive = args
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(GrepArgs {
            pattern: pattern.to_string(),
            path: path.to_string(),
            recursive,
            case_insensitive,
        })
    }

    /// Execute grep search
    fn execute_grep(&self, pattern: &str, base_path: &Path, recursive: bool, case_insensitive: bool) -> Result<Vec<GrepMatch>> {
        use std::sync::Mutex;

        // Compile regex pattern
        let regex = if case_insensitive {
            Regex::new(&format!("(?i){}", pattern))
        } else {
            Regex::new(pattern)
        }.map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;

        let matches = Arc::new(Mutex::new(Vec::new()));
        let fs = Arc::clone(&self.fs);

        // Check if it's a directory or file
        let is_dir = self.fs.is_dir(base_path);
        let is_file = self.fs.is_file(base_path);

        if !is_dir && !is_file {
            return Err(anyhow::anyhow!("Path does not exist"));
        }

        // Collect all files to search
        let mut files_to_search = Vec::new();

        if is_file {
            files_to_search.push(base_path.to_path_buf());
        } else if is_dir {
            // Use walkdir to collect files
            if recursive {
                for entry in walkdir::WalkDir::new(base_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        files_to_search.push(entry.path().to_path_buf());
                    }
                }
            } else {
                // Non-recursive: only files in current directory
                // We need to use a blocking approach for list_dir
                // For now, use a simple glob pattern
                use glob::glob;
                let pattern = format!("{}/*", base_path.display());
                for entry in glob(&pattern)
                    .map_err(|e| anyhow::anyhow!("Failed to read glob pattern: {}", e))?
                {
                    if let Ok(path) = entry {
                        if path.is_file() {
                            files_to_search.push(path);
                        }
                    }
                }
            }
        }

        // Search each file
        for file_path in files_to_search {
            // Use tokio runtime to block on async calls
            let content = {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(self.fs.read_file(&file_path))
                    .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file_path.display(), e))?
            };

            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    let mut matches_guard = matches.lock().unwrap();
                    matches_guard.push(GrepMatch {
                        file_path: file_path.to_string_lossy().to_string(),
                        line_number: line_num + 1,
                        line_content: line.to_string(),
                    });
                }
            }
        }

        let matches_guard = Arc::try_unwrap(matches)
            .map_err(|_| anyhow::anyhow!("Failed to extract matches"))?;
        let result = matches_guard.into_inner()?;

        Ok(result)
    }
}

/// Grep tool arguments
#[derive(Debug, Clone)]
struct GrepArgs {
    pattern: String,
    path: String,
    recursive: bool,
    case_insensitive: bool,
}

impl Tool for GrepTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "grep".to_string(),
            name: "grep".to_string(),
            description: "Search for text patterns in files using regular expressions".to_string(),
            category: Some("search".to_string()),
            parameters: serde_json::json!({
                "pattern": {
                    "type": "string",
                    "description": "Regular expression pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in (default: current directory)",
                    "default": "."
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Search recursively in subdirectories (default: false)",
                    "default": false
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive search (default: false)",
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
            let grep_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            let base_path = Path::new(&grep_args.path);

            // Check if path exists
            if !self.fs.exists(base_path) {
                let xml = XmlBuilder::build_error(
                    "grep",
                    "PATH_NOT_FOUND",
                    &format!("Path not found: {}", grep_args.path),
                    &format!("The path '{}' does not exist", grep_args.path),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("grep", xml));
            }

            // Execute grep (sync operation)
            let matches = self.execute_grep(&grep_args.pattern, base_path, grep_args.recursive, grep_args.case_insensitive)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Build XML response
            let xml = XmlBuilder::build_grep_result_xml(&grep_args.pattern, matches)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("grep", xml))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = serde_json::json!({"pattern": "TODO"});
        let parsed = GrepTool::parse_args(&args).unwrap();
        assert_eq!(parsed.pattern, "TODO");
        assert_eq!(parsed.path, ".");
        assert!(!parsed.recursive);
        assert!(!parsed.case_insensitive);

        let args = serde_json::json!({"pattern": "test", "path": "src", "recursive": true, "case_insensitive": true});
        let parsed = GrepTool::parse_args(&args).unwrap();
        assert_eq!(parsed.pattern, "test");
        assert!(parsed.recursive);
        assert!(parsed.case_insensitive);
    }
}
