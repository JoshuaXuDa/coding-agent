//! GrepTool - Regular expression content search
//!
//! Provides content-based file searching using regular expressions.
//! Returns matches with file paths, line numbers, and matching content.

use std::path::Path;
use std::fs::File;
use std::io::BufRead;
use tirea::prelude::{Tool, ToolDescriptor, ToolError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::tools::{ToolArgs, ToolContext, ToolExecutionEffect};

/// GrepTool - Content search using regular expressions
#[derive(Debug, Clone)]
pub struct GrepTool;

impl Tool for GrepTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "grep".to_string(),
            description: indoc::indoc!(r#"
                Search file contents using regular expressions. Returns matching lines with file paths and line numbers.
                This is a powerful tool for finding code patterns, function definitions, or specific text across files.

                Examples:
                - Find function definitions: pattern = "fn\s+\w+"
                - Find TODO comments: pattern = "(?i)todo"
                - Find specific word: pattern = "\bimport\b"
                - Case insensitive search: pattern = "(?i)error"

                Note: Uses Rust regex syntax. Use (?i) prefix for case-insensitive matching.
            "#).to_string(),
            parameters_schema: GrepParams::json_schema(),
        }
    }

    fn execute_effect(
        &self,
        args: ToolArgs,
        _context: &ToolContext,
    ) -> Result<ToolExecutionEffect, ToolError> {
        let params: GrepParams = serde_json::from_value(args.inner.into())
            .map_err(|e| ToolError::InvalidArgument(format!("Invalid arguments: {}", e)))?;

        let matches = search_files(&params.pattern, params.path.as_deref(), params.glob.as_deref())?;

        let result_text = if matches.is_empty() {
            format!("No matches found for pattern: {}", params.pattern)
        } else {
            let mut output = format!("Found {} match(es) for pattern '{}':\n", matches.len(), params.pattern);
            for m in &matches {
                output.push_str(&format!("{}:{}: {}\n", m.file_path, m.line_number, m.content));
            }
            output
        };

        Ok(result_text)
    }
}

/// Parameters for GrepTool
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct GrepParams {
    /// Regular expression pattern to search for
    pattern: String,

    /// Optional base directory to search in
    #[serde(default)]
    path: Option<String>,

    /// Optional glob pattern to filter files (e.g., "*.rs", "*.json")
    #[serde(default)]
    glob: Option<String>,
}

/// A single match result
#[derive(Debug)]
struct Match {
    file_path: String,
    line_number: usize,
    content: String,
}

/// Search files for regex pattern matches
fn search_files(
    pattern: &str,
    base_path: Option<&str>,
    file_filter: Option<&str>,
) -> Result<Vec<Match>, ToolError> {
    let regex = regex::Regex::new(pattern)
        .map_err(|e| ToolError::InvalidArgument(format!("Invalid regex: {}", e)))?;

    let base = Path::new(base_path.unwrap_or("."));

    let mut results = Vec::new();

    // Collect files to search
    let files_to_search: Vec<std::path::PathBuf> = if let Some(glob_pattern) = file_filter {
        // Use glob to find files
        let full_pattern = if glob_pattern.starts_with('/') || glob_pattern.starts_with("./") {
            glob_pattern.to_string()
        } else {
            format!("{}/{}", base.display(), glob_pattern)
        };

        glob::glob(&full_pattern)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid glob pattern: {}", e)))?
            .filter_map(|e| e.ok())
            .filter(|p| p.is_file())
            .collect()
    } else {
        // Walk directory recursively
        walkdir::WalkDir::new(base)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .collect()
    };

    // Search each file
    for file_path in files_to_search {
        if let Ok(file) = File::open(&file_path) {
            let reader = std::io::BufReader::new(file);

            for (line_num, line_result) in reader.lines().enumerate() {
                if let Ok(line_content) = line_result {
                    if regex.is_match(&line_content) {
                        results.push(Match {
                            file_path: file_path
                                .strip_prefix(".")
                                .ok()
                                .and_then(|p| p.to_str())
                                .unwrap_or_else(|| file_path.to_str().unwrap_or("<invalid>"))
                                .to_string(),
                            line_number: line_num + 1,
                            content: line_content,
                        });
                    }
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_files_with_temp_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();

        // Create test files
        std::fs::write(base_path.join("test1.rs"), "fn hello() {}\nfn world() {}\n// TODO: fix this\n").unwrap();
        std::fs::write(base_path.join("test2.rs"), "fn goodbye() {}\n// FIXME: bug\n").unwrap();

        // Search for function definitions
        let results = search_files(r"fn\s+\w+", Some(base_path.to_str().unwrap()), Some("*.rs")).unwrap();
        assert_eq!(results.len(), 3); // hello, world, goodbye
    }

    #[test]
    fn test_search_case_insensitive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("test.txt"), "ERROR: something\nerror: another\nWarning: check\n").unwrap();

        let results = search_files("(?i)error", Some(base_path.to_str().unwrap()), None).unwrap();
        assert_eq!(results.len(), 2); // Matches both ERROR and error
    }
}
