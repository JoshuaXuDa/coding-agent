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
use crate::tools::domain::{
    validation::{validate_regex_pattern, MAX_GREP_MATCHES},
    json_builder::{JsonBuilder, GrepMatch},
    error_handler::ErrorHandler,
    async_bridge::execute_blocking,
};

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

        // Validate pattern
        validate_regex_pattern(pattern)?;

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

    /// Execute grep search (async version)
    async fn execute_grep_async(
        &self,
        pattern: &str,
        base_path: &Path,
        recursive: bool,
        case_insensitive: bool,
    ) -> Result<Vec<GrepMatch>> {
        // Use execute_blocking to run the sync grep operation
        let pattern = pattern.to_string();
        let base_path = base_path.to_path_buf();
        let fs = Arc::clone(&self.fs);
        execute_blocking(move || {
            Self::execute_grep_sync_internal(&fs, &pattern, &base_path, recursive, case_insensitive)
        }).await
    }

    /// Execute grep search (sync version for CPU-bound work)
    fn execute_grep_sync(
        &self,
        pattern: &str,
        base_path: &Path,
        recursive: bool,
        case_insensitive: bool,
    ) -> Result<Vec<GrepMatch>> {
        Self::execute_grep_sync_internal(&self.fs, pattern, base_path, recursive, case_insensitive)
    }

    /// Internal static method for grep search (can be called from blocking context)
    fn execute_grep_sync_internal(
        fs: &Arc<dyn FileSystem>,
        pattern: &str,
        base_path: &Path,
        recursive: bool,
        case_insensitive: bool,
    ) -> Result<Vec<GrepMatch>> {
        use std::sync::Mutex;

        // Compile regex pattern
        let regex = if case_insensitive {
            Regex::new(&format!("(?i){}", pattern))
        } else {
            Regex::new(pattern)
        }.map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;

        let matches = Arc::new(Mutex::new(Vec::new()));
        let fs_clone = Arc::clone(fs);

        // Check if it's a directory or file
        let is_dir = fs.is_dir(base_path);
        let is_file = fs.is_file(base_path);

        if !is_dir && !is_file {
            return Err(anyhow::anyhow!("Path does not exist"));
        }

        // Collect all files to search (blocking operation)
        let files_to_search = {
            let mut files = Vec::new();

            if is_file {
                files.push(base_path.to_path_buf());
            } else if is_dir {
                // Use walkdir to collect files
                if recursive {
                    for entry in walkdir::WalkDir::new(base_path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            files.push(entry.path().to_path_buf());
                        }
                    }
                } else {
                    // Non-recursive: only files in current directory
                    use glob::glob;
                    let pattern = format!("{}/*", base_path.display());
                    for entry in glob(&pattern)
                        .map_err(|e| anyhow::anyhow!("Failed to read glob pattern: {}", e))?
                    {
                        if let Ok(path) = entry {
                            if path.is_file() {
                                files.push(path);
                            }
                        }
                    }
                }
            }

            files
        };

        // Search each file using blocking operations
        for file_path in &files_to_search {
            // Use blocking call for file reading
            let content = {
                // Get file content synchronously
                let fs_for_read = Arc::clone(&fs_clone);
                let file_path_clone = file_path.clone();
                std::thread::spawn(move || {
                    // Use tokio runtime for async fs call
                    let rt = tokio::runtime::Runtime::new()?;
                    let result = rt.block_on(fs_for_read.read_file(&file_path_clone));
                    drop(rt);
                    result
                }).join().map_err(|e| anyhow::anyhow!("Thread join error: {:?}", e))??
            };

            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    let mut matches_guard = matches.lock().unwrap();

                    // Check if we've exceeded maximum matches
                    if matches_guard.len() >= MAX_GREP_MATCHES {
                        break;
                    }

                    matches_guard.push(GrepMatch {
                        file_path: file_path.to_string_lossy().to_string(),
                        line_number: line_num + 1,
                        line_content: line.to_string(),
                    });
                }
            }

            // Stop if we've hit the limit
            let matches_guard = matches.lock().unwrap();
            if matches_guard.len() >= MAX_GREP_MATCHES {
                drop(matches_guard);
                break;
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
                .map_err(ErrorHandler::to_tool_error)?;

            let base_path = Path::new(&grep_args.path);

            // Check if path exists
            if !self.fs.exists(base_path) {
                let json = JsonBuilder::build_error(
                    "grep",
                    "PATH_NOT_FOUND",
                    &format!("Path not found: {}", grep_args.path),
                    &format!("The path '{}' does not exist", grep_args.path),
                ).map_err(ErrorHandler::to_tool_error)?;

                return Ok(ToolResult::success("grep", json));
            }

            // Execute grep (async operation)
            let matches = self.execute_grep_async(&grep_args.pattern, base_path, grep_args.recursive, grep_args.case_insensitive)
                .await
                .map_err(ErrorHandler::to_tool_error)?;

            // Build XML response
            let json = JsonBuilder::build_grep_results(&grep_args.pattern, matches)
                .map_err(ErrorHandler::to_tool_error)?;

            Ok(ToolResult::success("grep", json))
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

    #[test]
    fn test_parse_args_empty_pattern() {
        let args = serde_json::json!({"pattern": ""});
        assert!(GrepTool::parse_args(&args).is_err());

        let args = serde_json::json!({"pattern": "   "});
        assert!(GrepTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_catastrophic_pattern() {
        let args = serde_json::json!({"pattern": "a(*)"});
        assert!(GrepTool::parse_args(&args).is_err());

        let args = serde_json::json!({"pattern": "a(+)"});
        assert!(GrepTool::parse_args(&args).is_err());

        let args = serde_json::json!({"pattern": "a{100}"});
        assert!(GrepTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_valid_patterns() {
        let args = serde_json::json!({"pattern": "TODO"});
        assert!(GrepTool::parse_args(&args).is_ok());

        let args = serde_json::json!({"pattern": "[A-Z]+"});
        assert!(GrepTool::parse_args(&args).is_ok());

        let args = serde_json::json!({"pattern": "\\bword\\b"});
        assert!(GrepTool::parse_args(&args).is_ok());
    }
}
