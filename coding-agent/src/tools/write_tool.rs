//! WriteTool - Write entire file contents
//!
//! Provides atomic file writing capabilities.
//! Ensures parent directory exists before writing.

use std::path::Path;
use tirea::{Tool, ToolDescriptor};
use tirea_contract::{tool::{ToolArgs, ToolContext, ToolExecutionEffect}, ToolError, Value};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// WriteTool - File writing tool
#[derive(Debug, Clone)]
pub struct WriteTool;

impl Tool for WriteTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "write".to_string(),
            description: indoc::indoc!(r#"
                Write content to a file. Creates the file if it doesn't exist, overwrites if it does.
                Automatically creates parent directories if needed.

                IMPORTANT: This tool will overwrite existing files completely.
                Consider using the 'edit' tool for partial modifications.

                Use this tool when you need to:
                - Create a new file
                - Completely replace file contents
                - Write configuration files

                Examples:
                - Write to file: file_path = "src/utils.rs", content = "pub fn helper() {}"
                - Create config: file_path = "config/app.json", content = '{"key": "value"}'
            "#).to_string(),
            parameters_schema: WriteParams::json_schema(),
        }
    }

    fn execute_effect(
        &self,
        args: ToolArgs,
        _context: &ToolContext,
    ) -> Result<ToolExecutionEffect, ToolError> {
        let params: WriteParams = serde_json::from_value(args.inner.into())
            .map_err(|e| ToolError::InvalidArgument(format!("Invalid arguments: {}", e)))?;

        write_file(&params.file_path, &params.content)?;

        Ok(ToolExecutionEffect::simple_text(format!(
            "Successfully wrote {} bytes to {}",
            params.content.len(),
            params.file_path
        )))
    }
}

/// Parameters for WriteTool
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct WriteParams {
    /// Absolute path to the file to write
    file_path: String,

    /// Content to write to the file
    content: String,
}

/// Write content to a file
///
/// Creates parent directories if needed.
/// Uses atomic write strategy (write temp file then rename).
fn write_file(file_path: &str, content: &str) -> Result<(), ToolError> {
    let path = Path::new(file_path);

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::ExecutionFailed(format!(
                    "Failed to create parent directory: {}", e
                )))?;
        }
    }

    // Use atomic write strategy: write to temp file, then rename
    let temp_path = path.with_extension("tmp");

    // Write to temp file
    std::fs::write(&temp_path, content)
        .map_err(|e| ToolError::ExecutionFailed(format!(
            "Failed to write file: {}", e
        )))?;

    // Atomic rename
    std::fs::rename(&temp_path, path)
        .map_err(|e| ToolError::ExecutionFailed(format!(
            "Failed to finalize file write: {}", e
        )))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_file_creates_parent_dirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir
            .path()
            .join("nested/dir/test.txt")
            .to_str()
            .unwrap();

        write_file(file_path, "Hello, World!").unwrap();

        assert!(Path::new(file_path).exists());
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_write_file_overwrites() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path();

        std::fs::write(path, "Old content").unwrap();
        write_file(path.to_str().unwrap(), "New content").unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "New content");
    }

    #[test]
    fn test_write_empty_file() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path();

        write_file(path.to_str().unwrap(), "").unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "");
    }
}
