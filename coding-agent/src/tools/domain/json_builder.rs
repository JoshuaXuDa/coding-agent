//! JSON Builder domain service
//!
//! Provides standardized JSON generation for all tool outputs.
//! This service ensures consistent JSON schema across all tools.

use crate::platform::domain::filesystem::FileMetadata;
use serde_json::json;

/// JSON Builder domain service
///
/// Generates JSON output for tools with a consistent schema.
pub struct JsonBuilder;

impl JsonBuilder {
    /// Create a new JSON builder
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonBuilder {
    /// Build success response
    ///
    /// Creates a standardized success response with tool-specific data.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool
    /// * `data` - Tool-specific data as JSON value
    pub fn build_success(tool_name: &str, data: serde_json::Value) -> serde_json::Value {
        json!({
            "status": "success",
            "tool": tool_name,
            "data": data
        })
    }

    /// Build error response
    ///
    /// Creates a standardized error response.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool
    /// * `error_code` - Error code (e.g., "FILE_NOT_FOUND", "PERMISSION_DENIED")
    /// * `message` - Human-readable error message
    /// * `details` - Technical error details
    pub fn build_error(tool_name: &str, error_code: &str, message: &str, details: &str) -> serde_json::Value {
        json!({
            "status": "error",
            "tool": tool_name,
            "error": {
                "code": error_code,
                "message": message,
                "details": details
            }
        })
    }

    /// Build directory listing JSON
    ///
    /// Creates JSON output for the list tool.
    pub fn build_directory_listing(path: &str, entries: Vec<FileMetadata>) -> serde_json::Value {
        let file_count = entries.iter().filter(|e| e.file_type == crate::platform::domain::filesystem::FileType::File).count();
        let dir_count = entries.iter().filter(|e| e.file_type == crate::platform::domain::filesystem::FileType::Directory).count();

        let entries_json: Vec<serde_json::Value> = entries.iter().map(|entry| {
            let mut obj = json!({
                "name": entry.name,
                "type": match entry.file_type {
                    crate::platform::domain::filesystem::FileType::File => "file",
                    crate::platform::domain::filesystem::FileType::Directory => "directory",
                    crate::platform::domain::filesystem::FileType::Symlink => "symlink",
                    crate::platform::domain::filesystem::FileType::Other => "other",
                },
                "permissions": entry.permissions,
                "modified": entry.modified.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                "hidden": entry.is_hidden
            });

            // Add size for files only
            if entry.file_type == crate::platform::domain::filesystem::FileType::File {
                if let Some(obj_map) = obj.as_object_mut() {
                    obj_map.insert("size".to_string(), json!(entry.size));
                }
            }

            obj
        }).collect();

        let data = json!({
            "path": path,
            "entries": entries_json,
            "summary": {
                "files": file_count,
                "directories": dir_count,
                "total": entries.len()
            }
        });

        Self::build_success("list", data)
    }

    /// Build file content JSON
    ///
    /// Creates JSON output for the read tool.
    pub fn build_file_content(path: &str, content: &str, offset: Option<usize>, limit: Option<usize>, total_lines: Option<usize>) -> serde_json::Value {
        let mut data = json!({
            "path": path,
            "content": content,
            "size": content.len(),
            "lines_read": content.lines().count()
        });

        // Add total line count if available
        if let Some(total) = total_lines {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("total_lines".to_string(), json!(total));
            }
        }

        // Add range info if specified
        if offset.is_some() || limit.is_some() {
            let mut range_obj = json!({});
            if let Some(off) = offset {
                if let Some(obj) = range_obj.as_object_mut() {
                    obj.insert("offset".to_string(), json!(off));
                }
            }
            if let Some(lim) = limit {
                if let Some(obj) = range_obj.as_object_mut() {
                    obj.insert("limit".to_string(), json!(lim));
                }
            }
            if let Some(data_obj) = data.as_object_mut() {
                data_obj.insert("range".to_string(), range_obj);
            }
        }

        Self::build_success("read", data)
    }

    /// Build glob result JSON
    ///
    /// Creates JSON output for the glob tool.
    pub fn build_glob_results(pattern: &str, matches: Vec<String>) -> serde_json::Value {
        let data = json!({
            "pattern": pattern,
            "matches": matches,
            "count": matches.len()
        });

        Self::build_success("glob", data)
    }

    /// Build grep result JSON
    ///
    /// Creates JSON output for the grep tool.
    pub fn build_grep_results(pattern: &str, matches: Vec<GrepMatch>) -> serde_json::Value {
        let matches_json: Vec<serde_json::Value> = matches.iter().map(|m| {
            json!({
                "file": m.file_path,
                "line": m.line_number,
                "content": m.line_content
            })
        }).collect();

        let data = json!({
            "pattern": pattern,
            "matches": matches_json,
            "count": matches.len()
        });

        Self::build_success("grep", data)
    }

    /// Build command result JSON
    ///
    /// Creates JSON output for the bash tool.
    pub fn build_command_result(command: &str, result: &crate::platform::domain::command::CommandResult) -> serde_json::Value {
        let mut data = json!({
            "command": command,
            "success": result.success,
            "duration_ms": result.duration_ms
        });

        // Add exit code if available
        if let Some(code) = result.exit_code {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("exit_code".to_string(), json!(code));
            }
        }

        // Add stdout if present
        if !result.stdout.is_empty() {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("stdout".to_string(), json!(result.stdout));
            }
        }

        // Add stderr if present
        if !result.stderr.is_empty() {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("stderr".to_string(), json!(result.stderr));
            }
        }

        Self::build_success("bash", data)
    }

    /// Build write result JSON
    ///
    /// Creates JSON output for the write tool.
    pub fn build_write_result(path: &str, bytes_written: usize) -> serde_json::Value {
        let data = json!({
            "path": path,
            "bytes_written": bytes_written
        });

        Self::build_success("write", data)
    }

    /// Build stat result JSON
    ///
    /// Creates JSON output for the stat tool.
    pub fn build_stat_result(metadata: &FileMetadata) -> serde_json::Value {
        let mut data = json!({
            "path": metadata.path,
            "name": metadata.name,
            "type": match metadata.file_type {
                crate::platform::domain::filesystem::FileType::File => "file",
                crate::platform::domain::filesystem::FileType::Directory => "directory",
                crate::platform::domain::filesystem::FileType::Symlink => "symlink",
                crate::platform::domain::filesystem::FileType::Other => "other",
            },
            "permissions": metadata.permissions,
            "modified": metadata.modified.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "hidden": metadata.is_hidden,
            "readable": metadata.is_readable,
            "writable": metadata.is_writable
        });

        // Add size for files only
        if metadata.file_type == crate::platform::domain::filesystem::FileType::File {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("size".to_string(), json!(metadata.size));
            }
        }

        Self::build_success("stat", data)
    }

    /// Build PDF file JSON
    ///
    /// Creates JSON output for PDF files with text extraction and optional base64.
    pub fn build_pdf_result(path: &str, text_content: &str, base64_content: Option<&str>, metadata: &PdfMetadata) -> serde_json::Value {
        let mut pdf_metadata = json!({
            "size": metadata.size
        });

        // Add optional PDF metadata
        if let Some(pages) = metadata.pages {
            if let Some(obj) = pdf_metadata.as_object_mut() {
                obj.insert("pages".to_string(), json!(pages));
            }
        }
        if let Some(title) = &metadata.title {
            if let Some(obj) = pdf_metadata.as_object_mut() {
                obj.insert("title".to_string(), json!(title));
            }
        }
        if let Some(author) = &metadata.author {
            if let Some(obj) = pdf_metadata.as_object_mut() {
                obj.insert("author".to_string(), json!(author));
            }
        }

        let mut data = json!({
            "path": path,
            "type": "pdf",
            "metadata": pdf_metadata,
            "content": text_content
        });

        // Add base64 content if available
        if let Some(base64) = base64_content {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("content_base64".to_string(), json!(base64));
            }
        } else if metadata.size > 1_048_576 {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("note".to_string(), json!("Base64 encoding skipped due to size (> 1MB)"));
            }
        }

        Self::build_success("read", data)
    }

    /// Build image file JSON
    ///
    /// Creates JSON output for image files with base64 encoding and metadata.
    pub fn build_image_result(path: &str, base64_content: &str, metadata: &ImageMetadata) -> serde_json::Value {
        let mut img_metadata = json!({
            "format": metadata.format,
            "size": metadata.size
        });

        // Add optional image metadata
        if let Some(width) = metadata.width {
            if let Some(obj) = img_metadata.as_object_mut() {
                obj.insert("width".to_string(), json!(width));
            }
        }
        if let Some(height) = metadata.height {
            if let Some(obj) = img_metadata.as_object_mut() {
                obj.insert("height".to_string(), json!(height));
            }
        }
        if let Some(color_type) = &metadata.color_type {
            if let Some(obj) = img_metadata.as_object_mut() {
                obj.insert("color_type".to_string(), json!(color_type));
            }
        }
        if let Some(has_alpha) = metadata.has_alpha {
            if let Some(obj) = img_metadata.as_object_mut() {
                obj.insert("has_alpha".to_string(), json!(has_alpha));
            }
        }

        let data = json!({
            "path": path,
            "type": "image",
            "metadata": img_metadata,
            "content_base64": base64_content
        });

        Self::build_success("read", data)
    }

    /// Build binary file JSON
    ///
    /// Creates JSON output for binary files with base64 encoding.
    pub fn build_binary_result(path: &str, base64_content: &str, size: usize) -> serde_json::Value {
        let data = json!({
            "path": path,
            "type": "binary",
            "size": size,
            "encoding": "base64",
            "content_base64": base64_content
        });

        Self::build_success("read", data)
    }

    /// Build error for file too large for base64 encoding
    ///
    /// Creates an error response when a binary file is too large.
    pub fn build_binary_too_large_error(path: &str, size: usize, preview: &str) -> serde_json::Value {
        let details = format!(
            "File type: application/octet-stream\n\
             Size: {} bytes\n\
             Recommendation: Use specific file handler or process in chunks\n\
             Preview: {}",
            size,
            preview
        );

        Self::build_error(
            "read",
            "FILE_TOO_LARGE_FOR_BASE64",
            &format!("Binary file too large for base64 encoding: {}", path),
            &details,
        )
    }

    /// Build edit result JSON
    ///
    /// Creates JSON output for the edit tool.
    pub fn build_edit_result(path: &str, replacements: usize) -> serde_json::Value {
        let data = json!({
            "path": path,
            "replacements": replacements
        });

        Self::build_success("edit", data)
    }

    /// Build head/tail result JSON
    ///
    /// Creates JSON output for the head/tail tool.
    pub fn build_head_tail_result(path: &str, lines: Vec<String>, mode: &str, count: usize, total_lines: usize) -> serde_json::Value {
        let data = json!({
            "path": path,
            "mode": mode,
            "count": count,
            "total_lines": total_lines,
            "lines": lines
        });

        Self::build_success("head_tail", data)
    }
}

/// PDF metadata
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    pub pages: Option<u32>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub size: usize,
}

/// Image metadata
#[derive(Debug, Clone, Default)]
pub struct ImageMetadata {
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub color_type: Option<String>,
    pub has_alpha: Option<bool>,
    pub size: usize,
}

/// Grep match value object
#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::domain::filesystem::{FileType, FileMetadata};
    use chrono::Utc;

    #[test]
    fn test_build_success() {
        let json = JsonBuilder::build_success("test_tool", json!({"result": "ok"}));
        assert_eq!(json["status"], "success");
        assert_eq!(json["tool"], "test_tool");
        assert_eq!(json["data"]["result"], "ok");
    }

    #[test]
    fn test_build_error() {
        let json = JsonBuilder::build_error("test_tool", "FILE_NOT_FOUND", "File not found", "/path/to/file");
        assert_eq!(json["status"], "error");
        assert_eq!(json["tool"], "test_tool");
        assert_eq!(json["error"]["code"], "FILE_NOT_FOUND");
        assert_eq!(json["error"]["message"], "File not found");
        assert_eq!(json["error"]["details"], "/path/to/file");
    }

    #[test]
    fn test_build_file_content() {
        let json = JsonBuilder::build_file_content("/tmp/test.txt", "Hello, World!", None, None, Some(1));
        assert_eq!(json["status"], "success");
        assert_eq!(json["data"]["path"], "/tmp/test.txt");
        assert_eq!(json["data"]["content"], "Hello, World!");
        assert_eq!(json["data"]["total_lines"], 1);
        assert_eq!(json["data"]["lines_read"], 1);
        assert_eq!(json["data"]["size"], 13);
    }

    #[test]
    fn test_build_glob_results() {
        let matches = vec!["/tmp/file1.txt".to_string(), "/tmp/file2.txt".to_string()];
        let json = JsonBuilder::build_glob_results("*.txt", matches);
        assert_eq!(json["data"]["pattern"], "*.txt");
        assert_eq!(json["data"]["count"], 2);
        assert_eq!(json["data"]["matches"][0], "/tmp/file1.txt");
        assert_eq!(json["data"]["matches"][1], "/tmp/file2.txt");
    }

    #[test]
    fn test_build_directory_listing() {
        let entries = vec![
            FileMetadata {
                name: "file.txt".to_string(),
                path: "/tmp/file.txt".to_string(),
                file_type: FileType::File,
                size: 1024,
                permissions: "rw-r--r--".to_string(),
                modified: Utc::now(),
                is_hidden: false,
                is_readable: true,
                is_writable: true,
            }
        ];

        let json = JsonBuilder::build_directory_listing("/tmp", entries);
        assert_eq!(json["data"]["path"], "/tmp");
        assert_eq!(json["data"]["summary"]["files"], 1);
        assert_eq!(json["data"]["entries"][0]["name"], "file.txt");
        assert_eq!(json["data"]["entries"][0]["type"], "file");
        assert_eq!(json["data"]["entries"][0]["size"], 1024);
    }
}
