//! GlobTool - File pattern matching
//!
//! Provides file system pattern matching capabilities using glob patterns.
//! Results are sorted by modification time and limited to 100 files.

use std::path::Path;
use tirea::{Tool, ToolDescriptor};
use tirea_contract::{tool::{ToolArgs, ToolContext, ToolExecutionEffect}, ToolError, Value};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Maximum number of results to return
const MAX_RESULTS: usize = 100;

/// GlobTool - File pattern matching tool
#[derive(Debug, Clone)]
pub struct GlobTool;

impl Tool for GlobTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "glob".to_string(),
            description: indoc::indoc!(r#"
                Fast file pattern matching tool. Use this tool when you need to find files by name pattern.
                Returns matching file paths sorted by modification time (newest first). Maximum 100 results.

                Examples:
                - Find all Rust files: pattern = "**/*.rs"
                - Find all JSON files: pattern = "**/*.json"
                - Find in src directory: pattern = "src/**/*.rs"
                - Find specific file: pattern = "**/Cargo.toml"
            "#).to_string(),
            parameters_schema: GlobParams::json_schema(),
        }
    }

    fn execute_effect(
        &self,
        args: ToolArgs,
        _context: &ToolContext,
    ) -> Result<ToolExecutionEffect, ToolError> {
        // Parse arguments
        let params: GlobParams = serde_json::from_value(args.inner.into())
            .map_err(|e| ToolError::InvalidArgument(format!("Invalid arguments: {}", e)))?;

        // Execute glob pattern matching
        let matches = find_files(&params.pattern, params.path.as_deref())?;

        // Format result
        let result_text = if matches.is_empty() {
            format!("No files found matching pattern: {}", params.pattern)
        } else {
            let mut output = format!("Found {} file(s) matching '{}':\n", matches.len(), params.pattern);
            for file_path in &matches {
                output.push_str(&format!("  {}\n", file_path));
            }
            if matches.len() >= MAX_RESULTS {
                output.push_str(&format!("\n(Results limited to {} most recent files)\n", MAX_RESULTS));
            }
            output
        };

        Ok(ToolExecutionEffect::simple_text(result_text))
    }
}

/// Parameters for GlobTool
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct GlobParams {
    /// Glob pattern to match files (e.g., "**/*.rs", "src/**/*.json")
    pattern: String,

    /// Optional base directory to search in (defaults to current directory)
    #[serde(default)]
    path: Option<String>,
}

/// Find files matching a glob pattern
///
/// Returns files sorted by modification time (newest first).
/// Limited to MAX_RESULTS results.
fn find_files(pattern: &str, base_path: Option<&str>) -> Result<Vec<String>, ToolError> {
    let base = base_path
        .map(Path::new)
        .unwrap_or_else(|| Path::new("."));

    // Build the full pattern
    let full_pattern = if pattern.starts_with('/') || pattern.starts_with("./") {
        pattern.to_string()
    } else {
        format!("{}/{}", base.display(), pattern)
    };

    // Execute glob
    let mut matches: Vec<_> = glob::glob(&full_pattern)
        .map_err(|e| ToolError::ExecutionFailed(format!("Invalid glob pattern: {}", e)))?
        .filter_map(|entry| match entry {
            Ok(path) => {
                // Only include files, not directories
                if path.is_file() {
                    // Get modification time
                    path.metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .map(|mtime| (path, mtime))
                } else {
                    None
                }
            }
            Err(_) => None,
        })
        .collect();

    // Sort by modification time (newest first)
    matches.sort_by(|a, b| b.1.cmp(&a.1));

    // Extract just the paths, limit results
    let results: Vec<String> = matches
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(path, _)| {
            // Convert to relative path if possible
            path.strip_prefix(".")
                .ok()
                .and_then(|p| p.to_str())
                .unwrap_or_else(|| path.to_str().unwrap_or("<invalid>"))
                .to_string()
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_files_with_temp_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();

        // Create test files
        std::fs::write(base_path.join("test1.rs"), "content1").unwrap();
        std::fs::write(base_path.join("test2.rs"), "content2").unwrap();
        std::fs::write(base_path.join("test.txt"), "content3").unwrap();

        // Find all .rs files
        let results = find_files("*.rs", Some(base_path.to_str().unwrap())).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.ends_with("test1.rs")));
        assert!(results.iter().any(|p| p.ends_with("test2.rs")));
    }

    #[test]
    fn test_descriptor() {
        let tool = GlobTool;
        let desc = tool.descriptor();
        assert_eq!(desc.name, "glob");
        assert!(desc.description.contains("glob"));
        assert!(!desc.parameters_schema.is_empty());
    }
}
