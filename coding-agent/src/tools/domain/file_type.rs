//! File type detection and classification
//!
//! This module provides file type detection using both extension
//! and magic byte (content-based) detection for accurate classification.

use anyhow::Result;
use std::path::Path;

/// Maximum file size for base64 encoding (1MB)
pub const MAX_BASE64_SIZE: usize = 1_048_576;

/// File category for routing to appropriate handlers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    /// Text file (UTF-8 encoded)
    Text,
    /// PDF document
    Pdf,
    /// Image file
    Image,
    /// Markdown document
    Markdown,
    /// JSON data
    Json,
    /// Other binary file
    Binary,
}

impl FileCategory {
    /// Get the MIME type string for this category
    pub fn mime_type(&self) -> &'static str {
        match self {
            FileCategory::Text => "text/plain",
            FileCategory::Pdf => "application/pdf",
            FileCategory::Image => "image/*",
            FileCategory::Markdown => "text/markdown",
            FileCategory::Json => "application/json",
            FileCategory::Binary => "application/octet-stream",
        }
    }

    /// Whether this file type should be base64 encoded (if small enough)
    pub fn should_base64_encode(&self) -> bool {
        matches!(self, FileCategory::Pdf | FileCategory::Image | FileCategory::Binary)
    }

    /// Whether to extract text content from this file type
    pub fn should_extract_text(&self) -> bool {
        matches!(self, FileCategory::Pdf)
    }
}

/// File type detector
pub struct FileTypeDetector;

impl FileTypeDetector {
    /// Detect file category from path
    pub fn detect_from_path(path: &Path) -> Result<FileCategory> {
        // First try extension-based detection
        if let Some(category) = Self::detect_from_extension(path) {
            return Ok(category);
        }

        // Default to text for unknown extensions (will be validated on read)
        Ok(FileCategory::Text)
    }

    /// Detect file category from extension
    fn detect_from_extension(path: &Path) -> Option<FileCategory> {
        let extension = path.extension()?.to_str()?.to_lowercase();

        match extension.as_str() {
            // Text files (code, config, etc.)
            "txt" | "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "rb" | "go" | "java" | "c" | "cpp"
            | "h" | "hpp" | "cs" | "php" | "swift" | "kt" | "kts" | "scala" | "sh" | "bash" | "zsh"
            | "fish" | "ps1" | "bat" | "cmd" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf"
            | "xml" | "html" | "htm" | "css" | "scss" | "less" | "sql" | "r" | "m" | "lua"
            | "pl" | "pm" | "tcl" | "vb" | "fs" | "fsx" | "ex" | "exs" | "erl" | "hrl"
            | "dart" | "groovy" | "nim" | "nix" | "dhall" => Some(FileCategory::Text),

            // Markdown
            "md" | "markdown" => Some(FileCategory::Markdown),

            // JSON
            "json" | "jsonc" => Some(FileCategory::Json),

            // PDF
            "pdf" => Some(FileCategory::Pdf),

            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" | "tiff" | "tif"
            | "avif" | "jxl" | "heic" | "heif" => Some(FileCategory::Image),

            // Other known binary types
            "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" | "zst" | "exe" | "dll"
            | "so" | "dylib" | "bin" | "dat" | "db" | "sqlite" | "mdb" | "woff" | "woff2"
            | "ttf" | "otf" | "eot" | "mp3" | "mp4" | "avi" | "mov" | "wmv" | "flv"
            | "mkv" | "webm" | "wav" | "ogg" | "flac" | "aac" => Some(FileCategory::Binary),

            _ => None,
        }
    }

    /// Detect file category from content (magic bytes)
    pub fn detect_from_content(path: &Path, content: &[u8]) -> Result<FileCategory> {
        // PDF magic bytes: %PDF (25 50 44 46)
        if content.starts_with(b"%PDF-") {
            return Ok(FileCategory::Pdf);
        }

        // PNG magic bytes: 137 80 78 71 13 10 26 10
        if content.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Ok(FileCategory::Image);
        }

        // JPEG magic bytes: FF D8 FF
        if content.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Ok(FileCategory::Image);
        }

        // GIF magic bytes: GIF8
        if content.starts_with(b"GIF8") {
            return Ok(FileCategory::Image);
        }

        // BMP magic bytes: BM
        if content.starts_with(b"BM") {
            return Ok(FileCategory::Image);
        }

        // WebP magic bytes: RIFF....WEBP
        if content.len() > 12 && &content[0..4] == b"RIFF" && &content[8..12] == b"WEBP" {
            return Ok(FileCategory::Image);
        }

        // If content is valid UTF-8, treat as text
        if std::str::from_utf8(content).is_ok() {
            // Re-check with extension for more specific types
            if let Some(category) = Self::detect_from_extension(path) {
                return Ok(category);
            }
            return Ok(FileCategory::Text);
        }

        // Default to binary for unknown content
        Ok(FileCategory::Binary)
    }

    /// Check if file size is within base64 encoding limits
    pub fn can_encode_base64(size: usize, category: FileCategory) -> bool {
        match category {
            // Images always get base64 encoded (regardless of size, within reason)
            FileCategory::Image => size <= MAX_BASE64_SIZE * 5, // Allow up to 5MB for images
            // Small PDFs get base64 encoded
            FileCategory::Pdf => size <= MAX_BASE64_SIZE,
            // Small binaries get base64 encoded
            FileCategory::Binary => size <= MAX_BASE64_SIZE,
            // Text files don't need base64
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_from_extension_text() {
        let path = Path::new("test.rs");
        assert_eq!(FileTypeDetector::detect_from_extension(path), Some(FileCategory::Text));

        let path = Path::new("test.txt");
        assert_eq!(FileTypeDetector::detect_from_extension(path), Some(FileCategory::Text));
    }

    #[test]
    fn test_detect_from_extension_pdf() {
        let path = Path::new("document.pdf");
        assert_eq!(FileTypeDetector::detect_from_extension(path), Some(FileCategory::Pdf));
    }

    #[test]
    fn test_detect_from_extension_image() {
        let path = Path::new("image.png");
        assert_eq!(FileTypeDetector::detect_from_extension(path), Some(FileCategory::Image));

        let path = Path::new("photo.jpg");
        assert_eq!(FileTypeDetector::detect_from_extension(path), Some(FileCategory::Image));
    }

    #[test]
    fn test_detect_from_extension_markdown() {
        let path = Path::new("README.md");
        assert_eq!(FileTypeDetector::detect_from_extension(path), Some(FileCategory::Markdown));
    }

    #[test]
    fn test_detect_from_extension_json() {
        let path = Path::new("config.json");
        assert_eq!(FileTypeDetector::detect_from_extension(path), Some(FileCategory::Json));
    }

    #[test]
    fn test_detect_from_content_pdf() {
        let content = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        let path = Path::new("test.pdf");
        assert_eq!(FileTypeDetector::detect_from_content(path, content).unwrap(), FileCategory::Pdf);
    }

    #[test]
    fn test_detect_from_content_png() {
        let mut content = vec![0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        content.extend_from_slice(b"some data");
        let path = Path::new("test.png");
        assert_eq!(FileTypeDetector::detect_from_content(path, &content).unwrap(), FileCategory::Image);
    }

    #[test]
    fn test_detect_from_content_jpeg() {
        let content = vec![0xFFu8, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        let path = Path::new("test.jpg");
        assert_eq!(FileTypeDetector::detect_from_content(path, &content).unwrap(), FileCategory::Image);
    }

    #[test]
    fn test_detect_from_content_text() {
        let content = b"Hello, World!\nThis is a test file.";
        let path = Path::new("test.txt");
        assert_eq!(FileTypeDetector::detect_from_content(path, content).unwrap(), FileCategory::Text);
    }

    #[test]
    fn test_can_encode_base64() {
        assert!(FileTypeDetector::can_encode_base64(1024 * 512, FileCategory::Pdf)); // 512KB - OK
        assert!(!FileTypeDetector::can_encode_base64(1024 * 1024 * 2, FileCategory::Pdf)); // 2MB - Too big

        assert!(FileTypeDetector::can_encode_base64(1024 * 1024 * 3, FileCategory::Image)); // 3MB - OK for images
        assert!(!FileTypeDetector::can_encode_base64(1024 * 1024 * 6, FileCategory::Image)); // 6MB - Too big even for images
    }

    #[test]
    fn test_file_category_properties() {
        assert_eq!(FileCategory::Pdf.mime_type(), "application/pdf");
        assert_eq!(FileCategory::Text.mime_type(), "text/plain");

        assert!(FileCategory::Pdf.should_base64_encode());
        assert!(FileCategory::Image.should_base64_encode());
        assert!(FileCategory::Binary.should_base64_encode());
        assert!(!FileCategory::Text.should_base64_encode());

        assert!(FileCategory::Pdf.should_extract_text());
        assert!(!FileCategory::Image.should_extract_text());
    }
}
