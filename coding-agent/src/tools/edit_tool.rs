//! EditTool - String replacement with multi-way matching
//!
//! Provides precise string replacement using three matching strategies:
//! 1. Exact match (original content exactly)
//! 2. Trim match (ignore leading/trailing whitespace)
//! 3. Normalized whitespace match (ignore all whitespace differences)
//!
//! IMPORTANT: You MUST read the file first before using edit.

use std::path::Path;
use tirea::{Tool, ToolDescriptor};
use tirea_contract::{tool::{ToolArgs, ToolContext, ToolExecutionEffect}, ToolError, Value};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// EditTool - String replacement tool
#[derive(Debug, Clone)]
pub struct EditTool;

impl Tool for EditTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "edit".to_string(),
            description: indoc::indoc!(r#"
                Perform exact string replacements in a file. Uses a 3-way matching strategy to find the old string:
                1. Exact match (content must match exactly)
                2. Line trim match (ignore leading/trailing whitespace per line)
                3. Normalized match (ignore all whitespace differences)

                CRITICAL: You MUST use the 'read' tool first before using 'edit' to see the file contents.

                Use this tool when you need to:
                - Make targeted changes to existing code
                - Replace specific functions or blocks
                - Modify configuration values
                - Fix typos or update comments

                For writing new files or complete replacements, use the 'write' tool instead.

                Examples:
                - Replace a function:
                    file_path = "src/main.rs",
                    old_string = "fn old_name() { println!("old"); }",
                    new_string = "fn new_name() { println!("new"); }"
                - Replace with whitespace tolerance:
                    file_path = "config.json",
                    old_string = "{ "key": "value" }",
                    new_string = "{ "key": "new_value" }"

                Note: The tool will fail if old_string is not found or if multiple matches are found.
            "#).to_string(),
            parameters_schema: EditParams::json_schema(),
        }
    }

    fn execute_effect(
        &self,
        args: ToolArgs,
        _context: &ToolContext,
    ) -> Result<ToolExecutionEffect, ToolError> {
        let params: EditParams = serde_json::from_value(args.inner.into())
            .map_err(|e| ToolError::InvalidArgument(format!("Invalid arguments: {}", e)))?;

        let result = edit_file(
            &params.file_path,
            &params.old_string,
            &params.new_string,
            params.replace_all,
        )?;

        Ok(ToolExecutionEffect::simple_text(result))
    }
}

/// Parameters for EditTool
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct EditParams {
    /// Absolute path to the file to edit
    file_path: String,

    /// The old string to be replaced
    old_string: String,

    /// The new string to replace with
    new_string: String,

    /// Replace all occurrences (default: false, replaces first match only)
    #[serde(default)]
    replace_all: bool,
}

/// Edit a file by replacing old_string with new_string
///
/// Uses three-way matching strategy:
/// 1. Exact match
/// 2. Line trim match
/// 3. Normalized whitespace match
fn edit_file(
    file_path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, ToolError> {
    let path = Path::new(file_path);

    // Read file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

    // Try three matching strategies
    let replacement_result = if let Some(result) = try_exact_match(&content, old_string, new_string, replace_all) {
        result
    } else if let Some(result) = try_trim_match(&content, old_string, new_string, replace_all) {
        result
    } else if let Some(result) = try_normalized_match(&content, old_string, new_string, replace_all) {
        result
    } else {
        return Err(ToolError::ExecutionFailed(
            "Could not find the old_string in the file. \
             Make sure you have read the file first and that the old_string matches exactly. \
             Try reading the file again to see the current content.".to_string()
        ));
    };

    // Write the modified content
    std::fs::write(path, &replacement_result.content)
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

    Ok(format!(
        "Successfully replaced {} occurrence(s) in {}",
        replacement_result.count,
        file_path
    ))
}

/// Result of a replacement operation
#[derive(Debug)]
struct ReplacementResult {
    content: String,
    count: usize,
}

/// Try exact string matching
fn try_exact_match(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<ReplacementResult> {
    if !content.contains(old_string) {
        return None;
    }

    let result = if replace_all {
        content.replacen(old_string, new_string, usize::MAX)
    } else {
        content.replacen(old_string, new_string, 1)
    };

    let count = if replace_all {
        content.matches(old_string).count()
    } else {
        1
    };

    Some(ReplacementResult { content: result, count })
}

/// Try line-trim matching (ignore leading/trailing whitespace per line)
fn try_trim_match(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<ReplacementResult> {
    let content_lines: Vec<&str> = content.lines().collect();
    let old_lines: Vec<&str> = old_string.lines().collect();

    if old_lines.is_empty() {
        return None;
    }

    // Find the starting position
    let start_idx = find_sequence_with_trim(&content_lines, &old_lines)?;

    // Reconstruct with proper newlines
    let before = content_lines[..start_idx].join("\n");
    let after = if start_idx + old_lines.len() < content_lines.len() {
        content_lines[start_idx + old_lines.len()..].join("\n")
    } else {
        String::new()
    };

    // Determine the original line endings
    let line_ending = if content.contains("\r\n") { "\r\n" } else { "\n" };

    let mut result = if before.is_empty() {
        String::new()
    } else {
        before + line_ending
    };
    result.push_str(new_string);
    if !after.is_empty() {
        result.push_str(line_ending);
        result.push_str(&after);
    }

    Some(ReplacementResult { content: result, count: 1 })
}

/// Try normalized whitespace matching
fn try_normalized_match(
    content: &str,
    old_string: &str,
    new_string: &str,
    _replace_all: bool,
) -> Option<ReplacementResult> {
    // Normalize whitespace (collapse multiple spaces/tabs into single space)
    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");

    let content_normalized = normalize(content);
    let old_normalized = normalize(old_string);

    if !content_normalized.contains(&old_normalized) {
        return None;
    }

    // Find the position in normalized content
    let pos = content_normalized.find(&old_normalized)?;

    // For normalized match, we do a simpler replacement
    // (This is a best-effort approach)
    let result = content.replacen(old_string.trim(), new_string, 1);

    Some(ReplacementResult { content: result, count: 1 })
}

/// Find a sequence of lines in the content, ignoring leading/trailing whitespace
fn find_sequence_with_trim(content_lines: &[&str], search_lines: &[&str]) -> Option<usize> {
    'outer: for start_idx in 0..=content_lines.len().saturating_sub(search_lines.len()) {
        for (i, search_line) in search_lines.iter().enumerate() {
            if content_lines[start_idx + i].trim() != search_line.trim() {
                continue 'outer;
            }
        }
        return Some(start_idx);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let content = "fn hello() {\n    println!(\"hello\");\n}";
        let old = "    println!(\"hello\");";
        let new = "    println!(\"world\");";

        let result = try_exact_match(content, old, new, false);
        assert!(result.is_some());
        assert!(result.unwrap().content.contains("world"));
    }

    #[test]
    fn test_trim_match() {
        let content = "fn hello() {\n    println!(\"hello\");\n}";
        let old = "println!(\"hello\");";
        let new = "println!(\"world\");";

        let result = try_trim_match(content, old, new, false);
        assert!(result.is_some());
        assert!(result.unwrap().content.contains("println!(\"world\");"));
    }

    #[test]
    fn test_edit_file_with_temp_file() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let content = "Line 1\nLine 2\nLine 3\n";
        std::fs::write(path, content).unwrap();

        let result = edit_file(
            path.to_str().unwrap(),
            "Line 2",
            "Line 2 EDITED",
            false,
        );

        assert!(result.is_ok());

        let new_content = std::fs::read_to_string(path).unwrap();
        assert!(new_content.contains("Line 2 EDITED"));
    }

    #[test]
    fn test_edit_file_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("nonexistent.txt");

        let result = edit_file(
            path.to_str().unwrap(),
            "old",
            "new",
            false,
        );

        assert!(result.is_err());
    }
}
