//! Unix file system implementation
//!
//! Provides file system operations for Unix-like systems using standard
//! Rust library functions that work across all Unix platforms.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use crate::platform::domain::filesystem::{FileMetadata, FileType, FileSystem};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

/// Unix file system implementation
///
/// Uses standard Rust library functions to provide file system operations
/// on Unix-like systems (Linux, BSD, macOS).
pub struct UnixFileSystem;

impl UnixFileSystem {
    /// Create a new Unix file system instance
    pub fn new() -> Self {
        Self
    }

    /// Convert Unix mode to permissions string
    fn mode_to_permissions(mode: u32) -> String {
        let user = mode >> 6;
        let group = (mode >> 3) & 0o7;
        let other = mode & 0o7;

        format!(
            "{}{}{}{}{}{}{}{}{}",
            if user & 0o4 != 0 { 'r' } else { '-' },
            if user & 0o2 != 0 { 'w' } else { '-' },
            if user & 0o1 != 0 { 'x' } else { '-' },
            if group & 0o4 != 0 { 'r' } else { '-' },
            if group & 0o2 != 0 { 'w' } else { '-' },
            if group & 0o1 != 0 { 'x' } else { '-' },
            if other & 0o4 != 0 { 'r' } else { '-' },
            if other & 0o2 != 0 { 'w' } else { '-' },
            if other & 0o1 != 0 { 'x' } else { '-' },
        )
    }

    /// Convert file type from fs metadata
    fn file_type_from_metadata(ftype: std::fs::FileType, is_symlink: bool) -> FileType {
        if is_symlink {
            FileType::Symlink
        } else if ftype.is_file() {
            FileType::File
        } else if ftype.is_dir() {
            FileType::Directory
        } else {
            FileType::Other
        }
    }

    /// Check if a file is hidden (starts with .)
    fn is_hidden(name: &str) -> bool {
        name.starts_with('.') && name != "." && name != ".."
    }

    /// Convert metadata to FileMetadata
    fn metadata_to_file_metadata(path: &Path, name: String) -> Result<FileMetadata> {
        let metadata = fs::metadata(path)
            .context(format!("Failed to read metadata for: {}", path.display()))?;

        let is_symlink = fs::symlink_metadata(path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);

        let file_type = Self::file_type_from_metadata(metadata.file_type(), is_symlink);
        let permissions = Self::mode_to_permissions(metadata.permissions().mode());
        let modified = Utc.timestamp_opt(metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64, 0)
            .single()
            .unwrap_or_else(|| Utc::now());

        Ok(FileMetadata {
            name,
            path: path.to_string_lossy().to_string(),
            file_type,
            size: metadata.len(),
            permissions,
            modified,
            is_hidden: Self::is_hidden(path.file_name().and_then(|n| n.to_str()).unwrap_or("")),
            is_readable: metadata.permissions().mode() & 0o444 != 0,
            is_writable: metadata.permissions().mode() & 0o222 != 0,
        })
    }
}

#[async_trait]
impl FileSystem for UnixFileSystem {
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
    fn test_mode_to_permissions() {
        assert_eq!(UnixFileSystem::mode_to_permissions(0o644), "rw-r--r--");
        assert_eq!(UnixFileSystem::mode_to_permissions(0o755), "rwxr-xr-x");
        assert_eq!(UnixFileSystem::mode_to_permissions(0o600), "rw-------");
    }

    #[test]
    fn test_is_hidden() {
        assert!(UnixFileSystem::is_hidden(".hidden"));
        assert!(UnixFileSystem::is_hidden(".git"));
        assert!(!UnixFileSystem::is_hidden("visible.txt"));
        assert!(!UnixFileSystem::is_hidden("."));
        assert!(!UnixFileSystem::is_hidden(".."));
    }

    #[tokio::test]
    async fn test_read_file() {
        let fs = UnixFileSystem::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "Hello, World!").unwrap();

        let content = fs.read_file(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file() {
        let fs = UnixFileSystem::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs.write_file(&file_path, "Hello, World!").await.unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_list_dir() {
        let fs = UnixFileSystem::new();
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
        let fs = UnixFileSystem::new();
        let current = fs.current_dir().await.unwrap();
        assert!(!current.is_empty());
    }
}
