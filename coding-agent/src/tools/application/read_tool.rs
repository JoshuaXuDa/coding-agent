//! Read tool - Application layer
//!
//! Orchestrates file system operations to provide file reading functionality.

use anyhow::{anyhow, Result};
use base64::Engine as _;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::filesystem::FileSystem;
use crate::tools::domain::validation::{validate_path, validate_read_range};
use crate::tools::domain::json_builder::{JsonBuilder, ImageMetadata, PdfMetadata};
use crate::tools::domain::file_type::{FileCategory, FileTypeDetector};
use crate::tools::truncate_output;

/// Read tool
///
/// Provides file reading capabilities with range support.
pub struct ReadTool {
    /// File system service
    fs: Arc<dyn FileSystem>,
}

impl ReadTool {
    /// Create a new read tool
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<ReadArgs> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        // Validate path
        validate_path(path)?;

        let offset = args.get("offset").and_then(|v| v.as_u64()).map(|v| v as usize);
        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        // Validate offset and limit
        validate_read_range(offset, limit)?;

        Ok(ReadArgs {
            path: path.to_string(),
            offset,
            limit,
        })
    }

    /// Read file with range support
    async fn read_file_with_range(&self, path: &Path, offset: Option<usize>, limit: Option<usize>) -> Result<FileReadResult> {
        let content = self.fs.read_file(path).await?;

        // Calculate total line count before applying range filters
        let total_lines = content.lines().count();

        let result_content = match (offset, limit) {
            (Some(offset), Some(limit)) => {
                // Read from offset with limit
                let lines: Vec<&str> = content.lines().skip(offset).take(limit).collect();
                lines.join("\n")
            }
            (Some(offset), None) => {
                // Read from offset to end
                let lines: Vec<&str> = content.lines().skip(offset).collect();
                lines.join("\n")
            }
            (None, Some(limit)) => {
                // Read from start with limit
                let lines: Vec<&str> = content.lines().take(limit).collect();
                lines.join("\n")
            }
            (None, None) => {
                // Read entire file
                content
            }
        };

        Ok(FileReadResult {
            content: result_content,
            total_lines,
        })
    }
}

/// File read result
///
/// Contains both the content and metadata about the file.
#[derive(Debug, Clone)]
struct FileReadResult {
    /// The file content (possibly filtered by range)
    content: String,
    /// Total number of lines in the file
    total_lines: usize,
}

/// Read tool arguments
#[derive(Debug, Clone)]
struct ReadArgs {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

/// Extract text from PDF file
fn extract_pdf_text(data: &[u8]) -> Result<(String, PdfMetadata)> {
    use lopdf::Document;

    let doc = Document::load_mem(data)
        .map_err(|e| anyhow!("Failed to parse PDF: {}", e))?;

    // Get page count
    let pages = doc.get_pages().len() as u32;

    // Try to extract text from pages
    // This is a simplified approach - lopdf doesn't have built-in text extraction
    // For now, we'll just extract basic info
    let text_content = format!("[PDF Document with {} pages - text extraction not fully implemented]", pages);

    // Try to extract metadata from document info
    let mut title: Option<String> = None;
    let mut author: Option<String> = None;

    // Access trailer directly (public field in lopdf 0.31)
    let trailer = &doc.trailer;

    // Get Info dictionary
    if let Ok(info_obj) = trailer.get(b"Info") {
        if let Ok(info_ref) = info_obj.as_reference() {
            // Dereference the Info dictionary
            if let Ok(info) = doc.get_object(info_ref) {
                if let Ok(info_dict) = info.as_dict() {
                    // Extract Title - as_str returns Result<&[u8], lopdf::Error>
                    if let Ok(title_obj) = info_dict.get(b"Title") {
                        if let Ok(title_bytes) = title_obj.as_str() {
                            title = Some(String::from_utf8_lossy(title_bytes).to_string());
                        }
                    }
                    // Extract Author
                    if let Ok(author_obj) = info_dict.get(b"Author") {
                        if let Ok(author_bytes) = author_obj.as_str() {
                            author = Some(String::from_utf8_lossy(author_bytes).to_string());
                        }
                    }
                }
            }
        }
    }

    let pdf_metadata = PdfMetadata {
        pages: Some(pages),
        title,
        author,
        size: data.len(),
    };

    Ok((text_content, pdf_metadata))
}

/// Extract image metadata
fn extract_image_metadata(data: &[u8], format: &str) -> Result<ImageMetadata> {
    let mut metadata = ImageMetadata {
        format: format.to_string(),
        size: data.len(),
        ..Default::default()
    };

    // Try to get image dimensions using the image crate
    // For now, just set basic metadata
    // Full implementation would decode the image header
    match format {
        "png" => {
            // PNG width/height are at bytes 16-23 (big-endian)
            if data.len() >= 24 {
                let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
                let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
                metadata.width = Some(width);
                metadata.height = Some(height);
            }
            // PNG color type is at byte 25
            if data.len() >= 26 {
                let color_type = data[25];
                metadata.color_type = Some(match color_type {
                    0 => "Grayscale",
                    2 => "RGB",
                    3 => "Indexed",
                    4 => "Grayscale with Alpha",
                    6 => "RGBA",
                    _ => "Unknown",
                }.to_string());
                metadata.has_alpha = Some(color_type == 4 || color_type == 6);
            }
        }
        "jpeg" | "jpg" => {
            // JPEG is more complex, skip for now
            metadata.color_type = Some("YCbCr".to_string());
        }
        _ => {
            // Unknown format
        }
    }

    Ok(metadata)
}

/// Create hex preview of binary data
fn hex_preview(data: &[u8], max_bytes: usize) -> String {
    let bytes_to_show = data.len().min(max_bytes);
    let hex: Vec<String> = data[..bytes_to_show]
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect();
    hex.join(" ")
}

impl Tool for ReadTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "read".to_string(),
            name: "read".to_string(),
            description: "Read file contents with optional offset and limit for partial reading".to_string(),
            category: Some("filesystem".to_string()),
            parameters: serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "offset": {
                    "type": "number",
                    "description": "Skip this many lines from the start (default: 0)",
                    "default": 0
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of lines to read (default: read all)"
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
            let read_args = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            let path = Path::new(&read_args.path);

            // Check if path exists
            if !self.fs.exists(path) {
                let json = JsonBuilder::build_error(
                    "read",
                    "FILE_NOT_FOUND",
                    &format!("File not found: {}", read_args.path),
                    &format!("The file '{}' does not exist", read_args.path),
                );

                return Ok(ToolResult::success("read", json));
            }

            // Check if path is a file
            if !self.fs.is_file(path) {
                let json = JsonBuilder::build_error(
                    "read",
                    "NOT_A_FILE",
                    &format!("Not a file: {}", read_args.path),
                    &format!("The path '{}' is not a file", read_args.path),
                );

                return Ok(ToolResult::success("read", json));
            }

            // Detect file type
            let file_category = FileTypeDetector::detect_from_path(path)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Route to appropriate handler based on file type
            match file_category {
                FileCategory::Text | FileCategory::Markdown | FileCategory::Json => {
                    // Read as text
                    let read_result = self.read_file_with_range(path, read_args.offset, read_args.limit).await
                        .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                    // Truncate if too large
                    let content = truncate_output(&read_result.content);

                    // Build XML response with total line count
                    let json = JsonBuilder::build_file_content(
                        &read_args.path,
                        &content,
                        read_args.offset,
                        read_args.limit,
                        Some(read_result.total_lines),
                    );

                    Ok(ToolResult::success("read", json))
                }

                FileCategory::Pdf => {
                    // Read PDF as binary
                    let data = self.fs.read_file_binary(path).await
                        .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                    // Extract text and metadata
                    let (text_content, pdf_metadata) = extract_pdf_text(&data)
                        .unwrap_or_else(|e| (format!("[Error extracting PDF text: {}]", e), PdfMetadata {
                            pages: None,
                            title: None,
                            author: None,
                            size: data.len(),
                        }));

                    // Base64 encode if small enough
                    let base64_content = if FileTypeDetector::can_encode_base64(data.len(), file_category) {
                        Some(base64::engine::general_purpose::STANDARD.encode(&data))
                    } else {
                        None
                    };

                    // Build PDF XML
                    let json = JsonBuilder::build_pdf_result(
                        &read_args.path,
                        &text_content,
                        base64_content.as_deref(),
                        &pdf_metadata,
                    );

                    Ok(ToolResult::success("read", json))
                }

                FileCategory::Image => {
                    // Read image as binary
                    let data = self.fs.read_file_binary(path).await
                        .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                    // Extract image metadata
                    let format = path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("unknown");

                    let image_metadata = extract_image_metadata(&data, format)
                        .unwrap_or_else(|_| ImageMetadata {
                            format: format.to_string(),
                            size: data.len(),
                            ..Default::default()
                        });

                    // Base64 encode image
                    let base64_content = base64::engine::general_purpose::STANDARD.encode(&data);

                    // Build image XML
                    let json = JsonBuilder::build_image_result(
                        &read_args.path,
                        &base64_content,
                        &image_metadata,
                    );

                    Ok(ToolResult::success("read", json))
                }

                FileCategory::Binary => {
                    // Read binary file
                    let data = self.fs.read_file_binary(path).await
                        .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                    // Check if we can base64 encode
                    if FileTypeDetector::can_encode_base64(data.len(), file_category) {
                        // Base64 encode small binary
                        let base64_content = base64::engine::general_purpose::STANDARD.encode(&data);
                        let json = JsonBuilder::build_binary_result(
                            &read_args.path,
                            &base64_content,
                            data.len(),
                        );

                        Ok(ToolResult::success("read", json))
                    } else {
                        // File too large - return error with preview
                        let preview = hex_preview(&data, 32);
                        let json = JsonBuilder::build_binary_too_large_error(
                            &read_args.path,
                            data.len(),
                            &preview,
                        );

                        Ok(ToolResult::success("read", json))
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::create_filesystem;
    use crate::tools::domain::validation::{MAX_OFFSET, MAX_LIMIT};

    #[tokio::test]
    async fn test_parse_args() {
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let parsed = ReadTool::parse_args(&args).unwrap();
        assert_eq!(parsed.path, "/tmp/test.txt");
        assert!(parsed.offset.is_none());
        assert!(parsed.limit.is_none());

        let args = serde_json::json!({"path": "/tmp/test.txt", "offset": 10, "limit": 20});
        let parsed = ReadTool::parse_args(&args).unwrap();
        assert_eq!(parsed.offset, Some(10));
        assert_eq!(parsed.limit, Some(20));
    }

    #[test]
    fn test_parse_args_empty_path() {
        let args = serde_json::json!({"path": ""});
        assert!(ReadTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_path_traversal() {
        let args = serde_json::json!({"path": "../../etc/passwd"});
        assert!(ReadTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_invalid_range() {
        // Offset too large
        let args = serde_json::json!({"path": "/tmp/test.txt", "offset": MAX_OFFSET + 1});
        assert!(ReadTool::parse_args(&args).is_err());

        // Limit is zero
        let args = serde_json::json!({"path": "/tmp/test.txt", "limit": 0});
        assert!(ReadTool::parse_args(&args).is_err());

        // Limit too large
        let args = serde_json::json!({"path": "/tmp/test.txt", "limit": MAX_LIMIT + 1});
        assert!(ReadTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_valid_range() {
        let args = serde_json::json!({"path": "/tmp/test.txt", "offset": 1000, "limit": 50000});
        assert!(ReadTool::parse_args(&args).is_ok());
    }
}
