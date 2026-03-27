//! Context injector

use crate::context::domain::reference::{FileMetadata, InjectedContent};
use crate::platform::domain::filesystem::FileSystem;
use std::path::Path;
use std::sync::Arc;

/// Injects file content into the conversation context
pub struct ContextInjector {
    fs: Arc<dyn FileSystem>,
}

impl ContextInjector {
    /// Create a new context injector
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Inject a file and return its content with metadata
    pub async fn inject_file(&self, path: &Path) -> Result<InjectedContent, anyhow::Error> {
        let content = self.fs.read_file(path).await?;
        let language = Self::detect_language(path);
        let metadata = Self::build_metadata(&content);

        Ok(InjectedContent {
            path: path.to_path_buf(),
            content,
            language,
            metadata,
        })
    }

    /// Detect programming language from file extension
    fn detect_language(path: &Path) -> String {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| match ext {
                "rs" => "rust",
                "js" => "javascript",
                "ts" => "typescript",
                "tsx" => "typescript",
                "jsx" => "javascript",
                "py" => "python",
                "go" => "go",
                "java" => "java",
                "c" => "c",
                "cpp" | "cc" | "cxx" => "cpp",
                "h" => "c",
                "hpp" => "cpp",
                "cs" => "csharp",
                "php" => "php",
                "rb" => "ruby",
                "kt" => "kotlin",
                "swift" => "swift",
                "dart" => "dart",
                "scala" => "scala",
                "sh" => "bash",
                "bash" => "bash",
                "zsh" => "zsh",
                "fish" => "fish",
                "ps1" => "powershell",
                "sql" => "sql",
                "html" => "html",
                "css" => "css",
                "scss" => "scss",
                "less" => "less",
                "json" => "json",
                "yaml" | "yml" => "yaml",
                "toml" => "toml",
                "xml" => "xml",
                "md" => "markdown",
                "txt" => "text",
                "gitignore" => "gitignore",
                "dockerfile" => "dockerfile",
                _ => "text",
            })
            .unwrap_or("text")
            .to_string()
    }

    /// Build file metadata from content
    fn build_metadata(content: &str) -> FileMetadata {
        FileMetadata {
            total_lines: content.lines().count(),
            encoding: "utf-8".to_string(),
            size_bytes: content.len(),
        }
    }

    /// Format injected content for inclusion in message
    pub fn format_injected_content(content: &InjectedContent) -> String {
        format!(
            "\n\n<context:file path=\"{}\" language=\"{}\">\n<file_metadata>\n<total_lines>{}</total_lines>\n<size_bytes>{}</size_bytes>\n<encoding>{}</encoding>\n</file_metadata>\n\n{}\n</context:file>\n",
            content.path.display(),
            content.language,
            content.metadata.total_lines,
            content.metadata.size_bytes,
            content.metadata.encoding,
            content.content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_language() {
        assert_eq!(ContextInjector::detect_language(Path::new("main.rs")), "rust");
        assert_eq!(ContextInjector::detect_language(Path::new("app.js")), "javascript");
        assert_eq!(ContextInjector::detect_language(Path::new("lib.ts")), "typescript");
        assert_eq!(ContextInjector::detect_language(Path::new("script.py")), "python");
        assert_eq!(ContextInjector::detect_language(Path::new("data.json")), "json");
        assert_eq!(ContextInjector::detect_language(Path::new("README.md")), "markdown");
        assert_eq!(ContextInjector::detect_language(Path::new("unknown.xyz")), "text");
    }

    #[test]
    fn test_build_metadata() {
        let content = "line1\nline2\nline3";
        let metadata = ContextInjector::build_metadata(content);

        assert_eq!(metadata.total_lines, 3);
        assert_eq!(metadata.encoding, "utf-8");
        assert_eq!(metadata.size_bytes, content.len());
    }

    #[test]
    fn test_format_injected_content() {
        let content = InjectedContent {
            path: PathBuf::from("test.rs"),
            content: "fn main() {}".to_string(),
            language: "rust".to_string(),
            metadata: FileMetadata {
                total_lines: 1,
                encoding: "utf-8".to_string(),
                size_bytes: 12,
            },
        };

        let formatted = ContextInjector::format_injected_content(&content);

        assert!(formatted.contains("<context:file"));
        assert!(formatted.contains("path=\"test.rs\""));
        assert!(formatted.contains("language=\"rust\""));
        assert!(formatted.contains("<total_lines>1</total_lines>"));
        assert!(formatted.contains("fn main() {}"));
        assert!(formatted.contains("</context:file>"));
    }
}
