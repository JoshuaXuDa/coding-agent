//! Unix command execution implementation
//!
//! Provides command execution capabilities for Unix-like systems using
//! the standard process API.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use crate::platform::domain::command::{CommandExecutor, CommandRequest, CommandResult};

/// Unix command executor
///
/// Executes shell commands on Unix-like systems using tokio::process.
pub struct UnixCommandExecutor;

impl UnixCommandExecutor {
    /// Create a new Unix command executor
    pub fn new() -> Self {
        Self
    }

    /// Check if a command exists in PATH
    fn command_exists(command: &str) -> bool {
        // Use which command to check if command exists in PATH
        // This is more reliable than directly executing the command, as it
        // properly handles the PATH environment variable
        let result = Command::new("sh")
            .arg("-c")
            .arg(format!("which {}", command))
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status();

        result.map(|status| status.success()).unwrap_or(false)
    }
}

impl Default for UnixCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandExecutor for UnixCommandExecutor {
    async fn execute(&self, request: CommandRequest) -> Result<CommandResult> {
        let start = std::time::Instant::now();

        // Build the command
        // Use /usr/bin/env to ensure command is found in PATH
        let mut cmd = TokioCommand::new("/usr/bin/env");
        cmd.arg(&request.command);
        cmd.args(&request.args);

        // Set working directory if specified
        if let Some(dir) = &request.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Configure output capture
        if request.capture_stdout {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit());
        }

        if request.capture_stderr {
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stderr(Stdio::inherit());
        }

        // Execute the command
        let output = if let Some(timeout) = request.timeout {
            // Execute with timeout
            tokio::select! {
                result = cmd.output() => result,
                _ = tokio::time::sleep(timeout) => {
                    return Ok(CommandResult::terminated(
                        request.full_command(),
                        String::new(),
                        format!("Command timed out after {:?}", timeout),
                        start.elapsed().as_millis() as u64,
                    ));
                }
            }
        } else {
            cmd.output().await
        };

        let output = output.context(format!("Failed to execute command: {}", request.command))?;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Extract stdout and stderr
        let stdout = if request.capture_stdout {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            String::new()
        };

        let stderr = if request.capture_stderr {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::new()
        };

        // Get exit code
        let exit_code = output.status.code();

        // Build result
        let result = match exit_code {
            Some(0) => CommandResult::success(
                request.full_command(),
                0,
                stdout,
                stderr,
                duration_ms,
            ),
            Some(code) => CommandResult::failure(
                request.full_command(),
                code,
                stdout,
                stderr,
                duration_ms,
            ),
            None => CommandResult::terminated(
                request.full_command(),
                stdout,
                stderr,
                duration_ms,
            ),
        };

        Ok(result)
    }

    async fn execute_command_string(&self, command: &str) -> Result<CommandResult> {
        // For Unix, we can use sh -c to execute the command string
        let request = CommandRequest {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), command.to_string()],
            working_dir: None,
            env: HashMap::new(),
            timeout: None,
            capture_stdout: true,
            capture_stderr: true,
        };

        self.execute(request).await
    }

    fn is_available(&self, command: &str) -> bool {
        // Check for built-in commands
        if matches!(command, "sh" | "bash" | "zsh" | "dash") {
            return true;
        }

        // Check if command exists in PATH
        Self::command_exists(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
        let executor = UnixCommandExecutor::new();
        let request = CommandRequest::new("echo").with_args(vec!["Hello".to_string()]);

        let result = executor.execute(request).await.unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.trim(), "Hello");
    }

    #[tokio::test]
    async fn test_execute_command_string() {
        let executor = UnixCommandExecutor::new();
        let result = executor.execute_command_string("echo 'Hello, World!'").await.unwrap();

        assert!(result.success);
        assert_eq!(result.stdout.trim(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_execute_with_timeout() {
        let executor = UnixCommandExecutor::new();
        let request = CommandRequest::new("sleep")
            .with_args(vec!["0.1".to_string()])
            .with_timeout(Duration::from_secs(1));

        let result = executor.execute(request).await.unwrap();
        assert!(result.success);
        assert!(result.duration_ms < 1000);
    }

    #[tokio::test]
    async fn test_execute_with_timeout_exceeded() {
        let executor = UnixCommandExecutor::new();
        let request = CommandRequest::new("sleep")
            .with_args(vec!["10".to_string()])
            .with_timeout(Duration::from_millis(100));

        let result = executor.execute(request).await.unwrap();
        assert!(!result.success);
        assert!(result.stderr.contains("timed out"));
    }

    #[test]
    fn test_is_available() {
        let executor = UnixCommandExecutor::new();
        assert!(executor.is_available("sh"));
        assert!(executor.is_available("echo"));
        assert!(executor.is_available("ls"));
        assert!(!executor.is_available("nonexistent_command_xyz123"));
    }
}
