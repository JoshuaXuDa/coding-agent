//! Platform path value object
//!
//! Provides cross-platform path handling and normalization.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Platform enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    /// Microsoft Windows
    Windows,

    /// Unix-like systems (Linux, BSD, etc.)
    Unix,

    /// Apple macOS
    Macos,
}

impl Platform {
    /// Get the current platform
    pub fn current() -> Self {
        #[cfg(windows)]
        return Platform::Windows;

        #[cfg(target_os = "macos")]
        return Platform::Macos;

        #[cfg(unix)]
        return Platform::Unix;
    }

    /// Get the path separator for this platform
    pub fn path_separator(&self) -> char {
        match self {
            Platform::Windows => '\\',
            Platform::Unix | Platform::Macos => '/',
        }
    }

    /// Get the path separator string for this platform
    pub fn path_separator_str(&self) -> &str {
        match self {
            Platform::Windows => "\\",
            Platform::Unix | Platform::Macos => "/",
        }
    }

    /// Check if this platform uses drive letters
    pub fn uses_drive_letters(&self) -> bool {
        matches!(self, Platform::Windows)
    }
}

/// Platform path value object
///
/// Wraps a PathBuf and provides cross-platform utilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPath {
    inner: PathBuf,
    platform: Platform,
}

impl PlatformPath {
    /// Create a new platform path
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            inner: path.as_ref().to_path_buf(),
            platform: Platform::current(),
        }
    }

    /// Create a platform path from a string
    pub fn from_string(s: &str) -> Result<Self> {
        // Clean the path using path-clean crate
        let cleaned: String = path_clean::clean(s);

        Ok(Self {
            inner: cleaned.into(),
            platform: Platform::current(),
        })
    }

    /// Get the underlying path
    pub fn as_path(&self) -> &Path {
        &self.inner
    }

    /// Get the underlying PathBuf
    pub fn into_path_buf(self) -> PathBuf {
        self.inner
    }

    /// Get the platform
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Check if the path is absolute
    pub fn is_absolute(&self) -> bool {
        self.inner.is_absolute()
    }

    /// Check if the path is relative
    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    /// Get the file name
    pub fn file_name(&self) -> Option<&str> {
        self.inner
            .file_name()
            .and_then(|n| n.to_str())
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        self.inner.extension().and_then(|e| e.to_str())
    }

    /// Get the parent directory
    pub fn parent(&self) -> Option<Self> {
        self.inner.parent().map(|p| Self {
            inner: p.to_path_buf(),
            platform: self.platform,
        })
    }

    /// Get the file stem (name without extension)
    pub fn file_stem(&self) -> Option<&str> {
        self.inner.file_stem().and_then(|s| s.to_str())
    }

    /// Join this path with another component
    pub fn join(&self, component: impl AsRef<Path>) -> Self {
        Self {
            inner: self.inner.join(component),
            platform: self.platform,
        }
    }

    /// Normalize the path for display
    ///
    /// Converts to string with proper separators.
    pub fn to_display_string(&self) -> String {
        let path_str = self.inner.to_str().unwrap_or("");
        match self.platform {
            Platform::Windows => {
                path_str.replace('/', "\\")
            }
            Platform::Unix | Platform::Macos => {
                path_str.replace('\\', "/")
            }
        }
    }

    /// Convert to a string (lossy)
    pub fn to_string_lossy(&self) -> String {
        self.inner.to_string_lossy().to_string()
    }

    /// Check if this path starts with another path
    pub fn starts_with(&self, other: &PlatformPath) -> bool {
        self.inner.starts_with(&other.inner)
    }

    /// Create a path from a list of components
    pub fn from_components(components: &[&str]) -> Self {
        let mut path = PathBuf::new();
        for component in components {
            path.push(component);
        }

        Self {
            inner: path,
            platform: Platform::current(),
        }
    }

    /// Get the canonical (absolute) path
    ///
    /// # Errors
    /// - Returns error if path doesn't exist
    /// - Returns error if path contains invalid components
    pub fn canonicalize(&self) -> Result<Self> {
        let canonical = std::fs::canonicalize(&self.inner)
            .context(format!("Failed to canonicalize path: {}", self.inner.display()))?;

        Ok(Self {
            inner: canonical,
            platform: self.platform,
        })
    }

    /// Make the path absolute if it's relative
    ///
    /// # Errors
    /// - Returns error if current directory is inaccessible
    pub fn absolutize(&self) -> Result<Self> {
        if self.is_absolute() {
            return Ok(self.clone());
        }

        let current = std::env::current_dir()
            .context("Failed to get current directory")?;

        Ok(Self {
            inner: current.join(&self.inner),
            platform: self.platform,
        })
    }

    /// Get the relative path from this path to another
    pub fn relative_to(&self, base: &PlatformPath) -> Option<Self> {
        pathdiff::diff_paths(&self.inner, &base.inner).map(|path| Self {
            inner: path,
            platform: self.platform,
        })
    }
}

impl AsRef<Path> for PlatformPath {
    fn as_ref(&self) -> &Path {
        &self.inner
    }
}

impl PartialEq for PlatformPath {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for PlatformPath {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_current() {
        let platform = Platform::current();
        // Should not panic
        assert!([Platform::Windows, Platform::Unix, Platform::Macos].contains(&platform));
    }

    #[test]
    fn test_platform_path_new() {
        let path = PlatformPath::new("/tmp/test.txt");
        assert_eq!(path.to_string_lossy(), "/tmp/test.txt");
    }

    #[test]
    fn test_platform_path_from_string() {
        let path = PlatformPath::from_string("/tmp/test.txt").unwrap();
        assert_eq!(path.to_string_lossy(), "/tmp/test.txt");
    }

    #[test]
    fn test_platform_path_join() {
        let base = PlatformPath::new("/tmp");
        let full = base.join("test.txt");
        assert_eq!(full.to_string_lossy(), "/tmp/test.txt");
    }

    #[test]
    fn test_platform_path_file_name() {
        let path = PlatformPath::new("/tmp/test.txt");
        assert_eq!(path.file_name(), Some("test.txt"));
    }

    #[test]
    fn test_platform_path_extension() {
        let path = PlatformPath::new("/tmp/test.txt");
        assert_eq!(path.extension(), Some("txt"));
    }

    #[test]
    fn test_platform_path_parent() {
        let path = PlatformPath::new("/tmp/test.txt");
        let parent = path.parent().unwrap();
        assert_eq!(parent.to_string_lossy(), "/tmp");
    }

    #[test]
    fn test_platform_from_components() {
        let path = PlatformPath::from_components(&["tmp", "subdir", "file.txt"]);
        assert!(path.to_string_lossy().contains("tmp"));
        assert!(path.to_string_lossy().contains("subdir"));
        assert!(path.to_string_lossy().contains("file.txt"));
    }
}
