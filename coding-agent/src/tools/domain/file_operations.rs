//! File operation prechecker service
//!
//! Centralizes file validation logic for existence checks, type validation.
//! This eliminates repetitive file checking code across all tools.

use anyhow::{Result, anyhow};
use std::path::Path;
use std::sync::Arc;
use crate::platform::domain::filesystem::{FileSystem, FileType};

/// Information about file access
#[derive(Debug, Clone)]
pub struct FileAccessInfo {
    pub path: String,
    pub exists: bool,
    pub file_type: FileType,
}

/// Information about directory access
#[derive(Debug, Clone)]
pub struct DirectoryAccessInfo {
    pub path: String,
    pub exists: bool,
}

/// File operation errors
#[derive(Debug, Clone)]
pub enum FileOperationError {
    FileNotFound(String),
    NotAFile(String),
    NotADirectory(String),
    PathTooLong(String),
    InvalidPath(String),
    SpecialFileNotSupported(String),
}

impl std::fmt::Display for FileOperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOperationError::FileNotFound(path) => write!(f, "File not found: {}", path),
            FileOperationError::NotAFile(path) => write!(f, "Not a file: {}", path),
            FileOperationError::NotADirectory(path) => write!(f, "Not a directory: {}", path),
            FileOperationError::PathTooLong(path) => write!(f, "Path too long: {}", path),
            FileOperationError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            FileOperationError::SpecialFileNotSupported(path) => write!(f, "Special file not supported: {}", path),
        }
    }
}

impl std::error::Error for FileOperationError {}

/// File operation prechecker
///
/// Provides validation methods for file operations before they are executed.
pub struct FileOperationPrechecker {
    fs: Arc<dyn FileSystem>,
}

impl FileOperationPrechecker {
    /// Create a new prechecker
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Verify that a file exists
    pub async fn verify_file_readable(&self, path: &Path) -> Result<FileAccessInfo, FileOperationError> {
        let path_str = path.to_string_lossy().to_string();

        // Check if path exists
        if !self.fs.exists(path) {
            return Err(FileOperationError::FileNotFound(path_str));
        }

        // Check if it's a file
        if !self.fs.is_file(path) {
            return Err(FileOperationError::NotAFile(path_str));
        }

        Ok(FileAccessInfo {
            path: path_str,
            exists: true,
            file_type: FileType::File,
        })
    }

    /// Verify that a file exists
    pub async fn verify_file_writable(&self, path: &Path) -> Result<FileAccessInfo, FileOperationError> {
        let path_str = path.to_string_lossy().to_string();

        // Check if path exists
        if !self.fs.exists(path) {
            return Err(FileOperationError::FileNotFound(path_str));
        }

        // Check if it's a file
        if !self.fs.is_file(path) {
            return Err(FileOperationError::NotAFile(path_str));
        }

        Ok(FileAccessInfo {
            path: path_str,
            exists: true,
            file_type: FileType::File,
        })
    }

    /// Verify that a file exists (for operations that work with both files and directories)
    pub async fn verify_file_exists(&self, path: &Path) -> Result<FileAccessInfo, FileOperationError> {
        let path_str = path.to_string_lossy().to_string();

        // Check if path exists
        if !self.fs.exists(path) {
            return Err(FileOperationError::FileNotFound(path_str));
        }

        Ok(FileAccessInfo {
            path: path_str,
            exists: true,
            file_type: if self.fs.is_file(path) { FileType::File } else { FileType::Directory },
        })
    }

    /// Verify that a directory exists
    pub async fn verify_directory_exists(&self, path: &Path) -> Result<DirectoryAccessInfo, FileOperationError> {
        let path_str = path.to_string_lossy().to_string();

        // Check if path exists
        if !self.fs.exists(path) {
            return Err(FileOperationError::NotADirectory(path_str));
        }

        // Check if it's a directory
        if !self.fs.is_dir(path) {
            return Err(FileOperationError::NotADirectory(path_str));
        }

        Ok(DirectoryAccessInfo {
            path: path_str,
            exists: true,
        })
    }

    /// Verify that we can create a file at the given path
    pub async fn verify_can_create_file(&self, path: &Path) -> Result<(), FileOperationError> {
        // If file exists, check if it's a file
        if self.fs.exists(path) {
            if !self.fs.is_file(path) {
                let path_str = path.to_string_lossy().to_string();
                return Err(FileOperationError::NotAFile(path_str));
            }
        }

        Ok(())
    }

    /// Validate file type for a specific operation
    pub fn validate_file_type_for_operation(
        &self,
        file_type: FileType,
        operation: FileOperationType,
    ) -> Result<(), FileOperationError> {
        match operation {
            FileOperationType::Read => {
                if matches!(file_type, FileType::Other) {
                    return Err(FileOperationError::SpecialFileNotSupported(
                        "Cannot read special file (device, socket, etc.)".to_string()
                    ));
                }
            }
            FileOperationType::Write | FileOperationType::Edit => {
                if matches!(file_type, FileType::Directory) {
                    return Err(FileOperationError::NotAFile(
                        "Cannot write to directory".to_string()
                    ));
                }
                if matches!(file_type, FileType::Other) {
                    return Err(FileOperationError::SpecialFileNotSupported(
                        "Cannot write to special file".to_string()
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Types of file operations
#[derive(Debug, Clone, Copy)]
pub enum FileOperationType {
    Read,
    Write,
    Edit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_operation_error_display() {
        assert_eq!(
            format!("{}", FileOperationError::FileNotFound("/test".to_string())),
            "File not found: /test"
        );
        assert_eq!(
            format!("{}", FileOperationError::NotAFile("/test".to_string())),
            "Not a file: /test"
        );
        assert_eq!(
            format!("{}", FileOperationError::NotADirectory("/test".to_string())),
            "Not a directory: /test"
        );
        assert_eq!(
            format!("{}", FileOperationError::SpecialFileNotSupported("/dev/null".to_string())),
            "Special file not supported: /dev/null"
        );
    }

    #[test]
    fn test_validate_file_type_for_read() {
        let checker = FileOperationPrechecker {
            fs: unreachable!()
        };

        assert!(checker.validate_file_type_for_operation(FileType::File, FileOperationType::Read).is_ok());
        assert!(checker.validate_file_type_for_operation(FileType::Directory, FileOperationType::Read).is_ok());
        assert!(checker.validate_file_type_for_operation(FileType::Symlink, FileOperationType::Read).is_ok());

        let result = checker.validate_file_type_for_operation(FileType::Other, FileOperationType::Read);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FileOperationError::SpecialFileNotSupported(_)));
    }

    #[test]
    fn test_validate_file_type_for_write() {
        let checker = FileOperationPrechecker {
            fs: unreachable!()
        };

        assert!(checker.validate_file_type_for_operation(FileType::File, FileOperationType::Write).is_ok());
        assert!(checker.validate_file_type_for_operation(FileType::Symlink, FileOperationType::Write).is_ok());

        let result = checker.validate_file_type_for_operation(FileType::Directory, FileOperationType::Write);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FileOperationError::NotAFile(_)));

        let result = checker.validate_file_type_for_operation(FileType::Other, FileOperationType::Write);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FileOperationError::SpecialFileNotSupported(_)));
    }

    #[test]
    fn test_validate_file_type_for_edit() {
        let checker = FileOperationPrechecker {
            fs: unreachable!()
        };

        assert!(checker.validate_file_type_for_operation(FileType::File, FileOperationType::Edit).is_ok());

        let result = checker.validate_file_type_for_operation(FileType::Directory, FileOperationType::Edit);
        assert!(result.is_err());

        let result = checker.validate_file_type_for_operation(FileType::Other, FileOperationType::Edit);
        assert!(result.is_err());
    }
}
