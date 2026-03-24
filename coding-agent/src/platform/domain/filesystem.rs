//! FileSystem domain service
//!
//! This trait defines the interface for file system operations
//! that must be implemented for each platform.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// File metadata value object
///
/// Represents file system metadata in a platform-agnostic way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File or directory name
    pub name: String,

    /// Full path to the file
    pub path: String,

    /// Type of entry
    pub file_type: FileType,

    /// Size in bytes (0 for directories)
    pub size: u64,

    /// Permissions string (platform-specific format)
    pub permissions: String,

    /// Last modified timestamp
    pub modified: DateTime<Utc>,

    /// Whether the entry is hidden
    pub is_hidden: bool,

    /// Whether the entry is readable
    pub is_readable: bool,

    /// Whether the entry is writable
    pub is_writable: bool,
}

/// File type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// Regular file
    File,

    /// Directory
    Directory,

    /// Symbolic link
    Symlink,

    /// Other type (device, socket, etc.)
    Other,
}

/// FileSystem domain service trait
///
/// Provides cross-platform abstraction for file system operations.
/// Each platform (Unix, Windows) must implement this trait.
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Read file contents as a string
    ///
    /// # Errors
    /// - Returns error if file doesn't exist
    /// - Returns error if file is not readable
    /// - Returns error if content is not valid UTF-8
    async fn read_file(&self, path: &Path) -> Result<String>;

    /// Read file contents as raw bytes
    ///
    /// # Errors
    /// - Returns error if file doesn't exist
    /// - Returns error if file is not readable
    async fn read_file_binary(&self, path: &Path) -> Result<Vec<u8>>;

    /// Write content to a file
    ///
    /// Creates the file if it doesn't exist.
    /// Overwrites existing file content.
    ///
    /// # Errors
    /// - Returns error if directory doesn't exist
    /// - Returns error if file is not writable
    async fn write_file(&self, path: &Path, content: &str) -> Result<()>;

    /// List directory contents
    ///
    /// Returns metadata for each entry in the directory.
    ///
    /// # Errors
    /// - Returns error if path is not a directory
    /// - Returns error if directory is not readable
    async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>>;

    /// Get file or directory metadata
    ///
    /// # Errors
    /// - Returns error if path doesn't exist
    async fn file_metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Check if a path is a file
    fn is_file(&self, path: &Path) -> bool;

    /// Get current working directory
    ///
    /// # Errors
    /// - Returns error if working directory is inaccessible
    async fn current_dir(&self) -> Result<String>;

    /// Set current working directory
    ///
    /// # Errors
    /// - Returns error if directory doesn't exist
    /// - Returns error if directory is not accessible
    async fn set_current_dir(&self, path: &Path) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_serialization() {
        let file_type = FileType::File;
        let serialized = serde_json::to_string(&file_type).unwrap();
        assert_eq!(serialized, "\"File\"");

        let deserialized: FileType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, FileType::File);
    }

    #[test]
    fn test_file_metadata_serialization() {
        let metadata = FileMetadata {
            name: "test.txt".to_string(),
            path: "/tmp/test.txt".to_string(),
            file_type: FileType::File,
            size: 1024,
            permissions: "rw-r--r--".to_string(),
            modified: Utc::now(),
            is_hidden: false,
            is_readable: true,
            is_writable: true,
        };

        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: FileMetadata = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, metadata.name);
        assert_eq!(deserialized.size, metadata.size);
    }
}
