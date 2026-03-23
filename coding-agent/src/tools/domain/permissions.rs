//! Permission checker service
//!
//! Provides permission validation for file operations to prevent operations
//! that will fail due to insufficient permissions. This service gives helpful
//! error messages explaining permission issues.

use anyhow::{Result, anyhow};
use std::path::Path;
use std::sync::Arc;
use crate::platform::domain::filesystem::FileSystem;

/// Permission status for an operation
#[derive(Debug, Clone)]
pub struct PermissionStatus {
    pub allowed: bool,
    pub reason: Option<String>,
    pub suggestion: Option<String>,
}

impl PermissionStatus {
    /// Create a successful permission status
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            reason: None,
            suggestion: None,
        }
    }

    /// Create a denied permission status
    pub fn denied(reason: String, suggestion: Option<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason),
            suggestion,
        }
    }
}

/// Permission checker
///
/// Validates permissions before file operations are executed.
/// Note: This is a simplified version that doesn't check actual file permissions
/// due to limitations in the FileSystem trait. It validates path existence and type.
pub struct PermissionChecker {
    fs: Arc<dyn FileSystem>,
}

impl PermissionChecker {
    /// Create a new permission checker
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Check if a path can be read
    pub async fn check_read_permission(&self, path: &Path) -> Result<PermissionStatus> {
        // If path doesn't exist, we can't check permissions
        if !self.fs.exists(path) {
            return Ok(PermissionStatus::denied(
                format!("Path does not exist: {}", path.display()),
                Some("Check that the path is correct".to_string()),
            ));
        }

        // If path exists, assume readable (actual read will fail if not)
        Ok(PermissionStatus::allowed())
    }

    /// Check if a path can be written
    pub async fn check_write_permission(&self, path: &Path) -> Result<PermissionStatus> {
        // If path doesn't exist, check parent directory
        if !self.fs.exists(path) {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    return self.check_directory_write_permission(parent).await;
                }
            }
            // No parent (e.g., creating file in current directory)
            return Ok(PermissionStatus::allowed());
        }

        // Path exists, assume writable (actual write will fail if not)
        Ok(PermissionStatus::allowed())
    }

    /// Check if a directory can be created at the given path
    pub async fn check_directory_create_permission(&self, path: &Path) -> Result<PermissionStatus> {
        // If directory already exists, assume writable
        if self.fs.exists(path) {
            return Ok(PermissionStatus::allowed());
        }

        // Directory doesn't exist, check parent directory
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                return self.check_directory_write_permission(parent).await;
            }
        }

        Ok(PermissionStatus::allowed())
    }

    /// Check if we have write permission in a directory
    async fn check_directory_write_permission(&self, path: &Path) -> Result<PermissionStatus> {
        if !self.fs.exists(path) {
            return Ok(PermissionStatus::denied(
                format!("Parent directory does not exist: {}", path.display()),
                Some("Create the parent directory first".to_string()),
            ));
        }

        if !self.fs.is_dir(path) {
            return Ok(PermissionStatus::denied(
                format!("Parent path is not a directory: {}", path.display()),
                Some("Specify a valid directory path".to_string()),
            ));
        }

        // Assume writable (actual write will fail if not)
        Ok(PermissionStatus::allowed())
    }

    /// Generate a helpful explanation for permission denial
    fn explain_permission_denied(&self, _path: &Path, _operation: &str) -> Option<String> {
        // Platform-specific suggestions
        #[cfg(unix)]
        let suggestion = {
            "Try: chmod u+w <file> or run with appropriate permissions".to_string()
        };

        #[cfg(windows)]
        let suggestion = {
            "Check file properties and ensure you have write permissions".to_string()
        };

        #[cfg(not(any(unix, windows)))]
        let suggestion = {
            "Check file permissions".to_string()
        };

        Some(suggestion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_status_allowed() {
        let status = PermissionStatus::allowed();
        assert!(status.allowed);
        assert!(status.reason.is_none());
        assert!(status.suggestion.is_none());
    }

    #[test]
    fn test_permission_status_denied() {
        let status = PermissionStatus::denied(
            "Access denied".to_string(),
            Some("Check permissions".to_string()),
        );
        assert!(!status.allowed);
        assert_eq!(status.reason, Some("Access denied".to_string()));
        assert_eq!(status.suggestion, Some("Check permissions".to_string()));
    }

    #[test]
    fn test_permission_status_denied_no_suggestion() {
        let status = PermissionStatus::denied("Access denied".to_string(), None);
        assert!(!status.allowed);
        assert_eq!(status.reason, Some("Access denied".to_string()));
        assert!(status.suggestion.is_none());
    }
}
