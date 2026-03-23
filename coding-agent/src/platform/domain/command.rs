//! Command execution domain service
//!
//! This trait defines the interface for executing shell commands
//! in a cross-platform manner.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Command request value object
///
/// Represents a request to execute a command with configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Command to execute (e.g., "ls", "dir", "git status")
    pub command: String,

    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Working directory for command execution
    #[serde(default)]
    pub working_dir: Option<String>,

    /// Environment variables to set for the command
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Maximum time to wait for command to complete
    #[serde(default)]
    pub timeout: Option<Duration>,

    /// Whether to capture stdout
    #[serde(default = "default_capture")]
    pub capture_stdout: bool,

    /// Whether to capture stderr
    #[serde(default = "default_capture")]
    pub capture_stderr: bool,
}

fn default_capture() -> bool {
    true
}

impl CommandRequest {
    /// Create a new command request
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            timeout: None,
            capture_stdout: true,
            capture_stderr: true,
        }
    }

    /// Add arguments to the command
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set environment variables
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Get the full command string for display
    pub fn full_command(&self) -> String {
        if self.args.is_empty() {
            self.command.clone()
        } else {
            format!("{} {}", self.command, self.args.join(" "))
        }
    }
}

/// Command execution result value object
///
/// Represents the result of executing a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// The command that was executed
    pub command: String,

    /// Exit code (None if process was terminated)
    pub exit_code: Option<i32>,

    /// Standard output (if captured)
    #[serde(default)]
    pub stdout: String,

    /// Standard error (if captured)
    #[serde(default)]
    pub stderr: String,

    /// Whether the command succeeded (exit code 0)
    pub success: bool,

    /// Duration of command execution
    pub duration_ms: u64,

    /// Process ID (if available)
    #[serde(default)]
    pub pid: Option<u32>,
}

impl CommandResult {
    /// Create a successful command result
    pub fn success(
        command: String,
        exit_code: i32,
        stdout: String,
        stderr: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            command,
            exit_code: Some(exit_code),
            stdout,
            stderr,
            success: exit_code == 0,
            duration_ms,
            pid: None,
        }
    }

    /// Create a failed command result
    pub fn failure(
        command: String,
        exit_code: i32,
        stdout: String,
        stderr: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            command,
            exit_code: Some(exit_code),
            stdout,
            stderr,
            success: exit_code == 0,
            duration_ms,
            pid: None,
        }
    }

    /// Create a terminated command result
    pub fn terminated(command: String, stdout: String, stderr: String, duration_ms: u64) -> Self {
        Self {
            command,
            exit_code: None,
            stdout,
            stderr,
            success: false,
            duration_ms,
            pid: None,
        }
    }
}

/// CommandExecutor domain service trait
///
/// Provides cross-platform abstraction for executing shell commands.
/// Each platform (Unix, Windows) must implement this trait.
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Execute a command
    ///
    /// # Errors
    /// - Returns error if command cannot be spawned
    /// - Returns error if working directory doesn't exist
    async fn execute(&self, request: CommandRequest) -> Result<CommandResult>;

    /// Execute a command string (simple interface)
    ///
    /// Parses the command string and executes it.
    ///
    /// # Errors
    /// - Returns error if command string is invalid
    /// - Returns error if command cannot be spawned
    async fn execute_command_string(&self, command: &str) -> Result<CommandResult> {
        // Simple parsing: split by spaces
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty command string");
        }

        let request = CommandRequest {
            command: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
            working_dir: None,
            env: HashMap::new(),
            timeout: None,
            capture_stdout: true,
            capture_stderr: true,
        };

        self.execute(request).await
    }

    /// Check if a command is available
    fn is_available(&self, command: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_command_request() {
        let request = CommandRequest::new("ls")
            .with_args(vec!["-la".to_string(), "/tmp".to_string()])
            .with_timeout(Duration::from_secs(5));

        assert_eq!(request.command, "ls");
        assert_eq!(request.args.len(), 2);
        assert_eq!(request.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_full_command() {
        let request = CommandRequest::new("git")
            .with_args(vec!["status".to_string()]);

        assert_eq!(request.full_command(), "git status");
    }

    #[test]
    fn test_command_result_serialization() {
        let result = CommandResult::success(
            "ls".to_string(),
            0,
            "file1.txt\nfile2.txt".to_string(),
            "".to_string(),
            100,
        );

        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: CommandResult = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.command, "ls");
        assert_eq!(deserialized.exit_code, Some(0));
        assert!(deserialized.success);
    }
}
