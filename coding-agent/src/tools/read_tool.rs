//! ReadTool - Read file contents with range support
//!
//! Provides file reading capabilities with optional line range specification.
//! Supports offset/limit for reading specific sections of large files.

use std::path::Path;
use std::fs::File;
use std::io::BufRead;
use tirea::prelude::{Tool, ToolDescriptor, ToolError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::tools::{ToolArgs, ToolContext, ToolExecutionEffect};

/// Maximum lines to read without offset/limit
const MAX_DEFAULT_LINES: usize = 2000;

/// ReadTool - File reading tool
#[derive(Debug, Clone)]
pub struct ReadTool;

impl Tool for ReadTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "read".to_string(),
            description: indoc::indoc!(r#"
                Read file contents. Supports optional line range with offset/limit parameters.
                Returns content in cat -n format (with line numbers).

                Use this tool when you need to:
                - Read a specific file to understand its contents
                - View a section of a large file
                - Check file content before making edits

                For files larger than 2000 lines, use offset/limit to read specific sections.

                Examples:
                - Read entire file: file_path = "src/main.rs"
                - Read from line 100: file_path = "large.rs", offset = 99
                - Read 50 lines from line 100: file_path = "large.rs", offset = 99, limit = 50
            "#).to_string(),
            parameters_schema: ReadParams::json_schema(),
        }
    }

    fn execute_effect(
        &self,
        args: ToolArgs,
        _context: &ToolContext,
    ) -> Result<ToolExecutionEffect, ToolError> {
        let params: ReadParams = serde_json::from_value(args.inner.into())
            .map_err(|e| ToolError::InvalidArgument(format!("Invalid arguments: {}", e)))?;

        let content = read_file(
            &params.file_path,
            params.offset,
            params.limit,
        )?;

        Ok(content)
    }
}

/// Parameters for ReadTool
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ReadParams {
    /// Absolute path to the file to read
    file_path: String,

    /// Optional line offset (0-based, starts from that line)
    #[serde(default)]
    offset: Option<usize>,

    /// Optional maximum number of lines to read
    #[serde(default)]
    limit: Option<usize>,
}

/// Read a file with optional offset and limit
///
/// Returns content in cat -n format (line_number prefixed).
/// Automatically truncates output at MAX_DEFAULT_LINES.
fn read_file(file_path: &str, offset: Option<usize>, limit: Option<usize>) -> Result<String, ToolError> {
    let path = Path::new(file_path);

    // Check if file exists
    if !path.exists() {
        return Err(ToolError::ExecutionFailed(format!(
            "File not found: {}",
            file_path
        )));
    }

    // Check if it's a file (not directory)
    if !path.is_file() {
        return Err(ToolError::ExecutionFailed(format!(
            "Path is not a file: {}",
            file_path
        )));
    }

    let file = File::open(path)
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to open file: {}", e)))?;

    let reader = std::io::BufReader::new(file);
    let mut result = String::new();

    let start_line = offset.unwrap_or(0);
    let max_lines = limit.unwrap_or(MAX_DEFAULT_LINES);
    let end_line = if limit.is_some() {
        start_line + max_lines
    } else {
        usize::MAX
    };

    let mut current_line = 0;
    let mut lines_read = 0;

    for line_result in reader.lines() {
        if current_line < start_line {
            current_line += 1;
            continue;
        }

        if current_line >= end_line || lines_read >= max_lines {
            break;
        }

        let line_content = line_result
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read line: {}", e)))?;

        // Format: "     1\tcontent"
        result.push_str(&format!("{:>8}\t{}\n", current_line + 1, line_content));

        current_line += 1;
        lines_read += 1;

        // Auto-truncate if no explicit limit and we've read enough
        if limit.is_none() && lines_read >= MAX_DEFAULT_LINES {
            result.push_str(&format!(
                "\n--- File truncated at line {} (use offset/limit to read more) ---\n",
                current_line + 1
            ));
            break;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_with_temp_file() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";

        std::fs::write(temp_file.path(), content).unwrap();

        let result = read_file(temp_file.path().to_str().unwrap(), None, None).unwrap();
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 5"));
    }

    #[test]
    fn test_read_file_with_offset() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";

        std::fs::write(temp_file.path(), content).unwrap();

        let result = read_file(temp_file.path().to_str().unwrap(), Some(2), None).unwrap();
        // offset=2 means start from line 3 (0-based)
        assert!(!result.contains("Line 1"));
        assert!(!result.contains("Line 2"));
        assert!(result.contains("Line 3"));
    }

    #[test]
    fn test_read_file_with_limit() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";

        std::fs::write(temp_file.path(), content).unwrap();

        let result = read_file(temp_file.path().to_str().unwrap(), None, Some(3)).unwrap();
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 3"));
        assert!(!result.contains("Line 4"));
        assert!(!result.contains("Line 5"));
    }

    #[test]
    fn test_read_file_nonexistent() {
        let result = read_file("/nonexistent/file.txt", None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
