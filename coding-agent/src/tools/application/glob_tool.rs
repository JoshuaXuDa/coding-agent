//! Glob tool - Application layer
//!
//! Orchestrates file system operations to provide pattern matching functionality.

use anyhow::Result;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::xml_builder::XmlBuilder;

/// Glob tool
///
/// Provides file pattern matching capabilities using glob patterns.
pub struct GlobTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
}

impl GlobTool {
    /// Create a new glob tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<GlobArgs> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' argument"))?;

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        Ok(GlobArgs {
            pattern: pattern.to_string(),
            path: path.to_string(),
        })
    }

    /// Execute glob pattern matching
    fn execute_glob(&self, pattern: &str, base_path: &Path) -> Result<Vec<String>> {
        use glob::glob;

        let full_pattern = if base_path.as_os_str().is_empty() || base_path.to_str() == Some(".") {
            pattern.to_string()
        } else {
            format!("{}/{}", base_path.display(), pattern)
        };

        let mut matches = Vec::new();

        for entry in glob(&full_pattern)
            .map_err(|e| anyhow::anyhow!("Failed to read glob pattern: {}", e))?
        {
            match entry {
                Ok(path) => {
                    if let Some(path_str) = path.to_str() {
                        matches.push(path_str.to_string());
                    }
                }
                Err(e) => {
                    eprintln!("Glob error: {}", e);
                }
            }
        }

        // Sort matches
        matches.sort();

        Ok(matches)
    }
}

/// Glob tool arguments
#[derive(Debug, Clone)]
struct GlobArgs {
    pattern: String,
    path: String,
}

impl Tool for GlobTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "glob".to_string(),
            name: "glob".to_string(),
            description: "Find files matching a glob pattern (e.g., *.txt, src/**/*.rs)".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files (e.g., *.txt, src/**/*.rs)"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search in (default: current directory)",
                    "default": "."
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
            let glob_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            let base_path = Path::new(&glob_args.path);

            // Check if base path exists
            if !self.fs.exists(base_path) {
                let xml = XmlBuilder::build_error(
                    "glob",
                    "PATH_NOT_FOUND",
                    &format!("Base path not found: {}", glob_args.path),
                    &format!("The base path '{}' does not exist", glob_args.path),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("glob", xml));
            }

            // Execute glob
            let matches = self.execute_glob(&glob_args.pattern, base_path)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Build XML response
            let xml = XmlBuilder::build_glob_result_xml(&glob_args.pattern, matches)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("glob", xml))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = serde_json::json!({"pattern": "*.txt"});
        let parsed = GlobTool::parse_args(&args).unwrap();
        assert_eq!(parsed.pattern, "*.txt");
        assert_eq!(parsed.path, ".");

        let args = serde_json::json!({"pattern": "*.rs", "path": "src"});
        let parsed = GlobTool::parse_args(&args).unwrap();
        assert_eq!(parsed.pattern, "*.rs");
        assert_eq!(parsed.path, "src");
    }
}
