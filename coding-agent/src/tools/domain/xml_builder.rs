//! XML Builder domain service
//!
//! Provides standardized XML generation for all tool outputs.
//! This service ensures consistent XML schema across all tools.

use anyhow::Result;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;
use std::io::Cursor;
use crate::platform::domain::filesystem::FileMetadata;

/// XML Builder domain service
///
/// Generates XML output for tools with a consistent schema.
pub struct XmlBuilder;

impl XmlBuilder {
    /// Create a new XML builder
    pub fn new() -> Self {
        Self
    }
}

impl Default for XmlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlBuilder {
    /// Build success XML response
    ///
    /// Creates a standardized success response with tool-specific content.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool
    /// * `content` - Tool-specific XML content
    /// * `summary` - Human-readable summary
    pub fn build_success(tool_name: &str, content: &str, summary: &str) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // Root element
        let mut root = BytesStart::new("tool_result");
        root.push_attribute(("name", tool_name.as_ref()));
        root.push_attribute(("status", "success"));
        writer.write_event(Event::Start(root))?;

        // Tool-specific content
        writer.write_event(Event::Text(BytesText::new(content)))?;

        // Summary
        writer.write_event(Event::Start(BytesStart::new("summary")))?;
        writer.write_event(Event::Text(BytesText::new(summary)))?;
        writer.write_event(Event::End(BytesEnd::new("summary")))?;

        // Close root
        writer.write_event(Event::End(BytesEnd::new("tool_result")))?;

        let output = writer.into_inner().into_inner();
        Ok(String::from_utf8(output)?)
    }

    /// Build error XML response
    ///
    /// Creates a standardized error response.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool
    /// * `error_code` - Error code (e.g., "FILE_NOT_FOUND", "PERMISSION_DENIED")
    /// * `message` - Human-readable error message
    /// * `details` - Technical error details
    pub fn build_error(tool_name: &str, error_code: &str, message: &str, details: &str) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // Root element
        let mut root = BytesStart::new("tool_result");
        root.push_attribute(("name", tool_name.as_ref()));
        root.push_attribute(("status", "error"));
        writer.write_event(Event::Start(root))?;

        // Error code
        writer.write_event(Event::Start(BytesStart::new("error_code")))?;
        writer.write_event(Event::Text(BytesText::new(error_code)))?;
        writer.write_event(Event::End(BytesEnd::new("error_code")))?;

        // Message
        writer.write_event(Event::Start(BytesStart::new("message")))?;
        writer.write_event(Event::Text(BytesText::new(message)))?;
        writer.write_event(Event::End(BytesEnd::new("message")))?;

        // Details
        writer.write_event(Event::Start(BytesStart::new("details")))?;
        writer.write_event(Event::Text(BytesText::new(details)))?;
        writer.write_event(Event::End(BytesEnd::new("details")))?;

        // Close root
        writer.write_event(Event::End(BytesEnd::new("tool_result")))?;

        let output = writer.into_inner().into_inner();
        Ok(String::from_utf8(output)?)
    }

    /// Build directory listing XML
    ///
    /// Creates XML output for the list tool.
    pub fn build_directory_xml(path: &str, entries: Vec<FileMetadata>) -> Result<String> {
        let file_count = entries.iter().filter(|e| e.file_type == crate::platform::domain::filesystem::FileType::File).count();
        let dir_count = entries.iter().filter(|e| e.file_type == crate::platform::domain::filesystem::FileType::Directory).count();

        let mut content = String::new();

        // Directory element
        content.push_str(&format!("<directory path=\"{}\">", escape_xml(path)));

        // Entries
        for entry in &entries {
            content.push_str("<entry>");

            // Name
            content.push_str(&format!("<name>{}</name>", escape_xml(&entry.name)));

            // Type
            content.push_str(&format!("<type>{}</type>",
                match entry.file_type {
                    crate::platform::domain::filesystem::FileType::File => "file",
                    crate::platform::domain::filesystem::FileType::Directory => "directory",
                    crate::platform::domain::filesystem::FileType::Symlink => "symlink",
                    crate::platform::domain::filesystem::FileType::Other => "other",
                }
            ));

            // Size (only for files)
            if entry.file_type == crate::platform::domain::filesystem::FileType::File {
                content.push_str(&format!("<size>{}</size>", entry.size));
            }

            // Permissions
            content.push_str(&format!("<permissions>{}</permissions>", escape_xml(&entry.permissions)));

            // Modified timestamp
            content.push_str(&format!("<modified>{}</modified>", entry.modified.format("%Y-%m-%dT%H:%M:%SZ")));

            // Hidden flag
            content.push_str(&format!("<hidden>{}</hidden>", entry.is_hidden));

            // Close entry
            content.push_str("</entry>");
        }

        content.push_str("</directory>");

        // Summary
        let summary = format!("{} files, {} directories", file_count, dir_count);

        Self::build_success("list", &content, &summary)
    }

    /// Build file content XML
    ///
    /// Creates XML output for the read tool.
    pub fn build_file_content_xml(path: &str, content: &str, offset: Option<usize>, limit: Option<usize>) -> Result<String> {
        let mut content_xml = String::new();

        // File element
        content_xml.push_str(&format!("<file path=\"{}\">", escape_xml(path)));

        // Offset/limit if specified
        if offset.is_some() || limit.is_some() {
            content_xml.push_str("<range>");
            if let Some(off) = offset {
                content_xml.push_str(&format!("<offset>{}</offset>", off));
            }
            if let Some(lim) = limit {
                content_xml.push_str(&format!("<limit>{}</limit>", lim));
            }
            content_xml.push_str("</range>");
        }

        // Content (escaped)
        content_xml.push_str(&format!("<content>{}</content>", escape_xml(content)));

        // Size
        content_xml.push_str(&format!("<size>{}</size>", content.len()));

        content_xml.push_str("</file>");

        // Summary
        let lines = content.lines().count();
        let summary = format!("Read {} lines ({} bytes)", lines, content.len());

        Self::build_success("read", &content_xml, &summary)
    }

    /// Build glob result XML
    ///
    /// Creates XML output for the glob tool.
    pub fn build_glob_result_xml(pattern: &str, matches: Vec<String>) -> Result<String> {
        let count = matches.len();

        let mut content = String::new();

        // Matches element
        content.push_str(&format!("<matches pattern=\"{}\">", escape_xml(pattern)));

        // Match files
        for file_path in &matches {
            content.push_str(&format!("<file>{}</file>", escape_xml(file_path)));
        }

        content.push_str("</matches>");

        // Summary
        let summary = format!("Found {} files matching pattern", count);

        Self::build_success("glob", &content, &summary)
    }

    /// Build grep result XML
    ///
    /// Creates XML output for the grep tool.
    pub fn build_grep_result_xml(pattern: &str, matches: Vec<GrepMatch>) -> Result<String> {
        let count = matches.len();

        let mut content = String::new();

        // Matches element
        content.push_str(&format!("<matches pattern=\"{}\">", escape_xml(pattern)));

        // Match entries
        for m in &matches {
            content.push_str("<match>");
            content.push_str(&format!("<file>{}</file>", escape_xml(&m.file_path)));
            content.push_str(&format!("<line>{}</line>", m.line_number));
            content.push_str(&format!("<content>{}</content>", escape_xml(&m.line_content)));
            content.push_str("</match>");
        }

        content.push_str("</matches>");

        // Summary
        let summary = format!("Found {} matches", count);

        Self::build_success("grep", &content, &summary)
    }

    /// Build command result XML
    ///
    /// Creates XML output for the bash tool.
    pub fn build_command_result_xml(command: &str, result: &crate::platform::domain::command::CommandResult) -> Result<String> {
        let mut content = String::new();

        // Command element
        content.push_str(&format!("<command executed=\"{}\">", escape_xml(command)));

        // Exit code
        if let Some(code) = result.exit_code {
            content.push_str(&format!("<exit_code>{}</exit_code>", code));
        }

        // Success flag
        content.push_str(&format!("<success>{}</success>", result.success));

        // Duration
        content.push_str(&format!("<duration_ms>{}</duration_ms>", result.duration_ms));

        // Stdout (escaped)
        if !result.stdout.is_empty() {
            content.push_str(&format!("<stdout>{}</stdout>", escape_xml(&result.stdout)));
        }

        // Stderr (escaped)
        if !result.stderr.is_empty() {
            content.push_str(&format!("<stderr>{}</stderr>", escape_xml(&result.stderr)));
        }

        content.push_str("</command>");

        // Summary
        let summary = if result.success {
            format!("Command completed successfully ({}ms)", result.duration_ms)
        } else {
            format!("Command failed with exit code {:?} ({}ms)", result.exit_code, result.duration_ms)
        };

        Self::build_success("bash", &content, &summary)
    }

    /// Build write result XML
    ///
    /// Creates XML output for the write tool.
    pub fn build_write_result_xml(path: &str, bytes_written: usize) -> Result<String> {
        let content = format!(
            "<file path=\"{}\"><bytes_written>{}</bytes_written></file>",
            escape_xml(path),
            bytes_written
        );

        let summary = format!("Successfully wrote {} bytes to {}", bytes_written, path);

        Self::build_success("write", &content, &summary)
    }

    /// Build stat result XML
    ///
    /// Creates XML output for the stat tool.
    pub fn build_stat_result_xml(metadata: &FileMetadata) -> Result<String> {
        let mut content = String::new();

        content.push_str(&format!("<file path=\"{}\">", escape_xml(&metadata.path)));

        content.push_str(&format!("<name>{}</name>", escape_xml(&metadata.name)));

        content.push_str(&format!("<type>{}</type>",
            match metadata.file_type {
                crate::platform::domain::filesystem::FileType::File => "file",
                crate::platform::domain::filesystem::FileType::Directory => "directory",
                crate::platform::domain::filesystem::FileType::Symlink => "symlink",
                crate::platform::domain::filesystem::FileType::Other => "other",
            }
        ));

        if metadata.file_type == crate::platform::domain::filesystem::FileType::File {
            content.push_str(&format!("<size>{}</size>", metadata.size));
        }

        content.push_str(&format!("<permissions>{}</permissions>", escape_xml(&metadata.permissions)));
        content.push_str(&format!("<modified>{}</modified>", metadata.modified.format("%Y-%m-%dT%H:%M:%SZ")));
        content.push_str(&format!("<hidden>{}</hidden>", metadata.is_hidden));
        content.push_str(&format!("<readable>{}</readable>", metadata.is_readable));
        content.push_str(&format!("<writable>{}</writable>", metadata.is_writable));

        content.push_str("</file>");

        let summary = format!("File metadata: {}", metadata.name);

        Self::build_success("stat", &content, &summary)
    }
}

/// Grep match value object
#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
}

/// Escape special XML characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::domain::filesystem::{FileType, FileMetadata};
    use chrono::Utc;

    #[test]
    fn test_build_success() {
        let xml = XmlBuilder::build_success("test_tool", "<content>test</content>", "Test completed").unwrap();
        assert!(xml.contains("<tool_result"));
        assert!(xml.contains("name=\"test_tool\""));
        assert!(xml.contains("status=\"success\""));
        assert!(xml.contains("<summary>Test completed</summary>"));
    }

    #[test]
    fn test_build_error() {
        let xml = XmlBuilder::build_error("test_tool", "FILE_NOT_FOUND", "File not found", "/path/to/file").unwrap();
        assert!(xml.contains("status=\"error\""));
        assert!(xml.contains("<error_code>FILE_NOT_FOUND</error_code>"));
        assert!(xml.contains("<message>File not found</message>"));
        assert!(xml.contains("<details>/path/to/file</details>"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("test & <test>"), "test &amp; &lt;test&gt;");
        assert_eq!(escape_xml("quote's \"double\""), "quote&apos;s &quot;double&quot;");
    }

    #[test]
    fn test_build_directory_xml() {
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

        let xml = XmlBuilder::build_directory_xml("/tmp", entries).unwrap();
        assert!(xml.contains("<directory path=\"/tmp\">"));
        assert!(xml.contains("<name>file.txt</name>"));
        assert!(xml.contains("<type>file</type>"));
        assert!(xml.contains("<size>1024</size>"));
        assert!(xml.contains("1 files, 0 directories"));
    }

    #[test]
    fn test_build_file_content_xml() {
        let xml = XmlBuilder::build_file_content_xml("/tmp/test.txt", "Hello, World!", None, None).unwrap();
        assert!(xml.contains("<file path=\"/tmp/test.txt\">"));
        assert!(xml.contains("<content>Hello, World!</content>"));
        assert!(xml.contains("<size>13</size>"));
    }

    #[test]
    fn test_build_glob_result_xml() {
        let matches = vec!["/tmp/file1.txt".to_string(), "/tmp/file2.txt".to_string()];
        let xml = XmlBuilder::build_glob_result_xml("*.txt", matches).unwrap();
        assert!(xml.contains("<matches pattern=\"*.txt\">"));
        assert!(xml.contains("<file>/tmp/file1.txt</file>"));
        assert!(xml.contains("Found 2 files"));
    }
}
