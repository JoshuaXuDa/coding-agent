//! Common validation utilities for tools
//!
//! Provides reusable validation functions to ensure consistency
//! across all tools and prevent code duplication.

use anyhow::Result;

/// Maximum content size for write operations (100MB)
pub const MAX_CONTENT_SIZE: usize = 100 * 1024 * 1024;

/// Maximum number of grep matches
pub const MAX_GREP_MATCHES: usize = 10_000;

/// Maximum timeout for commands (10 minutes)
pub const MAX_TIMEOUT_SECS: u64 = 600;

/// Minimum timeout for commands (1 second)
pub const MIN_TIMEOUT_SECS: u64 = 1;

/// Maximum offset for read operations
pub const MAX_OFFSET: usize = 1_000_000;

/// Maximum limit for read operations
pub const MAX_LIMIT: usize = 100_000;

/// Validate a file path
///
/// Checks for:
/// - Empty paths
/// - Path traversal attempts (..)
/// - Null bytes
pub fn validate_path(path: &str) -> Result<()> {
    if path.trim().is_empty() {
        anyhow::bail!("Path cannot be empty");
    }

    if path.contains("..") {
        anyhow::bail!("Path contains '..' which may lead to directory traversal");
    }

    if path.contains('\0') {
        anyhow::bail!("Path contains null byte");
    }

    Ok(())
}

/// Validate a command string for security
///
/// Prevents command injection by checking for:
/// - Command chaining operators
/// - Command substitution
/// - Redirects
/// - Shell metacharacters
pub fn validate_command(command: &str) -> Result<()> {
    if command.trim().is_empty() {
        anyhow::bail!("Command cannot be empty");
    }

    let dangerous_patterns = ["&&", "||", "|", ";", "&", "$(", "`", ">", "<"];
    for pattern in &dangerous_patterns {
        if command.contains(pattern) {
            anyhow::bail!(
                "Command contains dangerous pattern '{}'. Command chaining and redirection are not allowed",
                pattern
            );
        }
    }

    let cmd_name = command.split_whitespace().next()
        .ok_or_else(|| anyhow::anyhow!("Command cannot be empty"))?;

    if !cmd_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/') {
        anyhow::bail!("Command name contains invalid characters: '{}'", cmd_name);
    }

    Ok(())
}

/// Validate command arguments
pub fn validate_command_args(args: &[String]) -> Result<()> {
    for arg in args {
        let dangerous_chars = ['$', '`', ';', '&', '|', '>', '<', '\n', '\r'];
        for char in &dangerous_chars {
            if arg.contains(*char) {
                anyhow::bail!(
                    "Argument contains dangerous character '{}': '{}'",
                    char, arg
                );
            }
        }
    }

    Ok(())
}

/// Validate timeout value
pub fn validate_timeout(timeout_secs: u64) -> Result<()> {
    if timeout_secs < MIN_TIMEOUT_SECS {
        anyhow::bail!("Timeout must be at least {} second", MIN_TIMEOUT_SECS);
    }

    if timeout_secs > MAX_TIMEOUT_SECS {
        anyhow::bail!(
            "Timeout cannot exceed {} seconds ({} minutes)",
            MAX_TIMEOUT_SECS,
            MAX_TIMEOUT_SECS / 60
        );
    }

    Ok(())
}

/// Validate content size for write operations
pub fn validate_content_size(content: &str) -> Result<()> {
    if content.is_empty() {
        anyhow::bail!("Content cannot be empty");
    }

    if content.len() > MAX_CONTENT_SIZE {
        anyhow::bail!(
            "Content size ({}) exceeds maximum allowed size of {} bytes ({} MB)",
            content.len(),
            MAX_CONTENT_SIZE,
            MAX_CONTENT_SIZE / (1024 * 1024)
        );
    }

    Ok(())
}

/// Validate regex pattern
pub fn validate_regex_pattern(pattern: &str) -> Result<()> {
    if pattern.trim().is_empty() {
        anyhow::bail!("Pattern cannot be empty");
    }

    // Check for potentially catastrophic patterns
    if pattern.contains("(*)") || pattern.contains("(+)") || pattern.contains("{100}") {
        anyhow::bail!("Pattern contains potentially catastrophic regex syntax");
    }

    Ok(())
}

/// Validate read offset and limit
pub fn validate_read_range(offset: Option<usize>, limit: Option<usize>) -> Result<()> {
    if let Some(off) = offset {
        if off > MAX_OFFSET {
            anyhow::bail!("Offset {} is too large. Maximum is {}", off, MAX_OFFSET);
        }
    }

    if let Some(lim) = limit {
        if lim == 0 {
            anyhow::bail!("Limit cannot be zero");
        }
        if lim > MAX_LIMIT {
            anyhow::bail!("Limit {} is too large. Maximum is {}", lim, MAX_LIMIT);
        }
    }

    Ok(())
}

/// Validate that a string is not empty or just whitespace
pub fn validate_non_empty_string(value: &str, field_name: &str) -> Result<()> {
    if value.trim().is_empty() {
        anyhow::bail!("{} cannot be empty", field_name);
    }
    Ok(())
}

/// File operation types for validation
#[derive(Debug, Clone, Copy)]
pub enum FileOperationType {
    Read,
    Write,
    Edit,
}

/// Validate file type for a specific operation
///
/// Ensures that the file type is appropriate for the operation being performed.
pub fn validate_file_type_for_operation(
    file_type: crate::platform::domain::filesystem::FileType,
    operation: FileOperationType,
) -> Result<()> {
    match operation {
        FileOperationType::Read => {
            if matches!(file_type, crate::platform::domain::filesystem::FileType::Other) {
                anyhow::bail!("Cannot read special file (device, socket, etc.)");
            }
        }
        FileOperationType::Write | FileOperationType::Edit => {
            if matches!(file_type, crate::platform::domain::filesystem::FileType::Directory) {
                anyhow::bail!("Cannot write to directory");
            }
            if matches!(file_type, crate::platform::domain::filesystem::FileType::Other) {
                anyhow::bail!("Cannot write to special file (device, socket, etc.)");
            }
        }
    }
    Ok(())
}

/// Maximum symlink depth to prevent infinite loops
pub const MAX_SYMLINK_DEPTH: usize = 8;

/// Validate symlink safety
///
/// Checks for symlink depth to prevent infinite loops and detects
/// potential symlink attacks.
pub fn validate_symlink_safe(current_depth: usize) -> Result<()> {
    if current_depth >= MAX_SYMLINK_DEPTH {
        anyhow::bail!(
            "Symlink depth exceeds maximum allowed depth of {} (possible circular symlink)",
            MAX_SYMLINK_DEPTH
        );
    }
    Ok(())
}

/// Validate path is not a symlink attack vector
///
/// Checks for suspicious patterns that might indicate symlink attacks.
pub fn validate_symlink_security(path: &str) -> Result<()> {
    // Check for path traversal that might be used in symlink attacks
    if path.contains("../") || path.contains("..\\") {
        anyhow::bail!("Path contains parent directory references that may be unsafe with symlinks");
    }

    // Check for absolute path symlinks that could be redirected
    if path.starts_with('/') && path.contains("/tmp/") {
        // This is a heuristic - /tmp symlinks can be manipulated by other users
        anyhow::bail!("Path may be vulnerable to symlink attacks in /tmp");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path() {
        assert!(validate_path("").is_err());
        assert!(validate_path("../etc").is_err());
        assert!(validate_path("/tmp/../etc").is_err());
        assert!(validate_path("/tmp/file.txt").is_ok());
    }

    #[test]
    fn test_validate_command() {
        assert!(validate_command("").is_err());
        assert!(validate_command("ls && rm -rf /").is_err());
        assert!(validate_command("ls").is_ok());
        assert!(validate_command("git status").is_ok());
    }

    #[test]
    fn test_validate_timeout() {
        assert!(validate_timeout(0).is_err());
        assert!(validate_timeout(700).is_err());
        assert!(validate_timeout(30).is_ok());
    }

    #[test]
    fn test_validate_content_size() {
        assert!(validate_content_size("").is_err());
        assert!(validate_content_size("Hello").is_ok());
        assert!(validate_content_size(&"x".repeat(MAX_CONTENT_SIZE + 1)).is_err());
    }

    #[test]
    fn test_validate_read_range() {
        assert!(validate_read_range(Some(MAX_OFFSET + 1), None).is_err());
        assert!(validate_read_range(None, Some(0)).is_err());
        assert!(validate_read_range(Some(100), Some(1000)).is_ok());
    }

    #[test]
    fn test_validate_file_type_for_read() {
        use crate::platform::domain::filesystem::FileType;

        assert!(validate_file_type_for_operation(FileType::File, FileOperationType::Read).is_ok());
        assert!(validate_file_type_for_operation(FileType::Directory, FileOperationType::Read).is_ok());
        assert!(validate_file_type_for_operation(FileType::Symlink, FileOperationType::Read).is_ok());
        assert!(validate_file_type_for_operation(FileType::Other, FileOperationType::Read).is_err());
    }

    #[test]
    fn test_validate_file_type_for_write() {
        use crate::platform::domain::filesystem::FileType;

        assert!(validate_file_type_for_operation(FileType::File, FileOperationType::Write).is_ok());
        assert!(validate_file_type_for_operation(FileType::Directory, FileOperationType::Write).is_err());
        assert!(validate_file_type_for_operation(FileType::Other, FileOperationType::Write).is_err());
    }

    #[test]
    fn test_validate_file_type_for_edit() {
        use crate::platform::domain::filesystem::FileType;

        assert!(validate_file_type_for_operation(FileType::File, FileOperationType::Edit).is_ok());
        assert!(validate_file_type_for_operation(FileType::Directory, FileOperationType::Edit).is_err());
        assert!(validate_file_type_for_operation(FileType::Other, FileOperationType::Edit).is_err());
    }

    #[test]
    fn test_validate_symlink_safe() {
        assert!(validate_symlink_safe(0).is_ok());
        assert!(validate_symlink_safe(4).is_ok());
        assert!(validate_symlink_safe(7).is_ok());
        assert!(validate_symlink_safe(8).is_err());
        assert!(validate_symlink_safe(10).is_err());
    }

    #[test]
    fn test_validate_symlink_security() {
        assert!(validate_symlink_security("/home/user/file.txt").is_ok());
        assert!(validate_symlink_security("../etc/passwd").is_err());
        assert!(validate_symlink_security("path\\..\\file").is_err());
        assert!(validate_symlink_security("/tmp/suspicious/link").is_err());
    }
}
