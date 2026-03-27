//! Platform factory
//!
//! Provides factory functions to create platform-specific implementations
//! based on the current operating system.

use std::sync::Arc;
use crate::platform::domain::filesystem::FileSystem;
use crate::platform::domain::command::CommandExecutor;

#[cfg(unix)]
use crate::platform::platform_impl::unix::{UnixFileSystem, UnixCommandExecutor};

#[cfg(windows)]
use crate::platform::platform_impl::windows::{WindowsFileSystem, WindowsCommandExecutor};

/// Create a file system implementation for the current platform
///
/// This factory function automatically selects the correct implementation
/// based on the operating system.
///
/// # Returns
///
/// An `Arc<dyn FileSystem>` with the appropriate platform-specific implementation.
///
/// # Example
///
/// ```rust
/// use coding_agent::platform::create_filesystem;
///
/// let fs = create_filesystem();
/// let contents = fs.read_file(Path::new("README.md")).await?;
/// ```
pub fn create_filesystem() -> Arc<dyn FileSystem> {
    #[cfg(unix)]
    {
        Arc::new(UnixFileSystem::new())
    }

    #[cfg(windows)]
    {
        Arc::new(WindowsFileSystem::new())
    }
}

/// Create a command executor for the current platform
///
/// This factory function automatically selects the correct implementation
/// based on the operating system.
///
/// # Returns
///
/// An `Arc<dyn CommandExecutor>` with the appropriate platform-specific implementation.
///
/// # Example
///
/// ```rust
/// use coding_agent::platform::create_command_executor;
///
/// let executor = create_command_executor();
/// let result = executor.execute_command_string("ls -la").await?;
/// ```
pub fn create_command_executor() -> Arc<dyn CommandExecutor> {
    #[cfg(unix)]
    {
        Arc::new(UnixCommandExecutor::new())
    }

    #[cfg(windows)]
    {
        Arc::new(WindowsCommandExecutor::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_create_filesystem() {
        let fs = create_filesystem();
        // Should not panic
        assert!(fs.exists(Path::new(".")));
    }

    #[test]
    fn test_create_command_executor() {
        let executor = create_command_executor();
        // Built-in commands should always be available
        #[cfg(unix)]
        assert!(executor.is_available("sh"));

        #[cfg(windows)]
        assert!(executor.is_available("cmd"));
    }
}
