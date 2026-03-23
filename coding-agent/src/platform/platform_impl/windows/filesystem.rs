//! Windows file system implementation
//!
//! Provides file system operations for Windows using standard
//! Rust library functions that work on Windows.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::platform::domain::filesystem::{FileMetadata, FileType, FileSystem};
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::Path;

/// Windows file system implementation
///
/// Uses standard Rust library functions to provide file system operations
/// on Windows systems.
pub struct WindowsFileSystem;

impl WindowsFileSystem {
    /// Create a new Windows file system instance
    pub fn new() -> Self {
        Self
    }

    /// Convert Windows attributes to permissions string
    fn attributes_to_permissions(attrs: u32) -> String {
        let mut result = String::new();

        // Read-only
        if attrs & 0x1 != 0 {
            result.push('r');
        } else {
            result.push('w');
        }

        // Hidden
        if attrs & 0x2 != 0 {
            result.push('h');
        } else {
            result.push('-');
        }

        // System
        if attrs & 0x4 != 0 {
            result.push('s');
        } else {
            result.push('-');
        }

        // Archive
        if attrs & 0x20 != 0 {
            result.push('a');
        } else {
            result.push('-');
        }

        result
    }

    /// Convert file type from fs metadata
    fn file_type_from_metadata(ftype: std::fs::FileType) -> FileType {
        if ftype.is_file() {
            FileType::File
        } else if ftype.is_dir() {
            FileType::Directory
        } else {
            FileType::Other
        }
    }

    /// Check if a file is hidden on Windows
    fn is_hidden(path: &Path, attrs: u32) -> bool {
        // Check if hidden attribute is set
        if attrs & 0x2 != 0 {
            return true;
        }

        // Also check if name starts with . (Unix-style hidden files)
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str.starts_with('.') && name_str != "." && name_str != ".." {
                    return true;
                }
            }
        }

        false
    }

    /// Convert metadata to FileMetadata
    fn metadata_to_file_metadata(path: &Path, name: String) -> Result<FileMetadata> {
        let metadata = fs::metadata(path)
            .context(format!("Failed to read metadata for: {}", path.display()))?;

        let file_type = Self::file_type_from_metadata(metadata.file_type());
        let attrs = metadata.file_attributes();
        let permissions = Self::attributes_to_permissions(attrs);

        // Convert Windows file time to DateTime
        let last_write = metadata.last_write_time();
        // Windows file time is 100-nanosecond intervals since 1601-01-01
        let unix_timestamp = (last_write / 10_000_000) - 11_644_473_600;
        let modified = DateTime::timestamp_millis(&Utc, unix_timestamp * 1000);

        Ok(FileMetadata {
            name,
            path: path.to_string_lossy().to_string(),
            file_type,
            size: metadata.len(),
            permissions,
            modified,
            is_hidden: Self::is_hidden(path, attrs),
            is_readable: true, // Windows doesn't have a simple readable check
            is_writable: attrs & 0x1 == 0, // Not read-only
        })
    }
}

#[async_trait]
impl FileSystem for WindowsFileSystem {
    async fn read_file(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)
            .context(format!("Failed to read file: {}", path.display()))?;
        Ok(content)
    }

    async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create directory: {}", parent.display()))?;
            }
        }

        fs::write(path, content)
            .context(format!("Failed to write file: {}", path.display()))?;
        Ok(())
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let entries = fs::read_dir(path)
            .context(format!("Failed to read directory: {}", path.display()))?;

        let mut results = Vec::new();

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            match Self::metadata_to_file_metadata(&entry_path, name) {
                Ok(metadata) => results.push(metadata),
                Err(e) => {
                    eprintln!("Warning: Failed to get metadata for {}: {}", entry_path.display(), e);
                }
            }
        }

        // Sort by name (directories first, then files)
        results.sort_by(|a, b| {
            if a.file_type == FileType::Directory && b.file_type != FileType::Directory {
                std::cmp::Ordering::Less
            } else if a.file_type != FileType::Directory && b.file_type == FileType::Directory {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        Ok(results)
    }

    async fn file_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        Self::metadata_to_file_metadata(path, name)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    async fn current_dir(&self) -> Result<String> {
        let current = std::env::current_dir()
            .context("Failed to get current directory")?;
        Ok(current.to_string_lossy().to_string())
    }

    async fn set_current_dir(&self, path: &Path) -> Result<()> {
        std::env::set_current_dir(path)
            .context(format!("Failed to set current directory: {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attributes_to_permissions() {
        // Read-only, hidden, system, archive
        assert_eq!(WindowsFileSystem::attributes_to_permissions(0x27), "rhsa");
        // Normal file
        assert_eq!(WindowsFileSystem::attributes_to_permissions(0x20), "w---");
        // Read-only
        assert_eq!(WindowsFileSystem::attributes_to_permissions(0x01), "r---");
    }

    #[tokio::test]
    async fn test_read_file() {
        let fs = WindowsFileSystem::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!").unwrap();

        let content = fs.read_file(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file() {
        let fs = WindowsFileSystem::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs.write_file(&file_path, "Hello, World!").await.unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_list_dir() {
        let fs = WindowsFileSystem::new();
        let temp_dir = tempfile::tempdir().unwrap();

        // Create some test files
        fs::create_file(temp_dir.path().join("file1.txt")).unwrap();
        fs::create_file(temp_dir.path().join("file2.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let entries = fs.list_dir(temp_dir.path()).await.unwrap();
        assert_eq!(entries.len(), 3);

        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"file1.txt"));
        assert!(names.contains(&"file2.txt"));
        assert!(names.contains(&"subdir"));
    }

    #[tokio::test]
    async fn test_current_dir() {
        let fs = WindowsFileSystem::new();
        let current = fs.current_dir().await.unwrap();
        assert!(!current.is_empty());
    }
}
