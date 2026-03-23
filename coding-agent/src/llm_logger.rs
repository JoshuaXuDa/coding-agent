//! LLM Interaction Logger
//!
//! Provides structured logging for all LLM interactions including requests,
//! responses, tool calls, and errors. Logs are stored in JSON format for
//! easy parsing and debugging.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{DateTime, Utc};

/// LLM interaction logger
pub struct LlmLogger {
    log_file: BufWriter<File>,
    round: usize,
}

impl LlmLogger {
    /// Create a new LLM logger
    ///
    /// Creates the logs directory if it doesn't exist and opens
    /// the log file for appending.
    pub fn new() -> Result<Self, std::io::Error> {
        let log_dir = Path::new("logs");
        if !log_dir.exists() {
            std::fs::create_dir_all(log_dir)?;
        }

        let log_path = log_dir.join("llm_interactions.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            log_file: BufWriter::new(file),
            round: 0,
        })
    }

    /// Log an LLM request
    pub fn log_request(&mut self, user_message: &str) -> Result<(), std::io::Error> {
        self.round += 1;

        let entry = LogEntry {
            timestamp: Utc::now(),
            entry_type: LogType::Request,
            round: self.round,
            user_message: Some(user_message.to_string()),
            llm_request: None,
            llm_response: None,
            tool_calls: None,
            duration_ms: None,
            error: None,
        };

        self.write_entry(&entry)
    }

    /// Log an LLM response
    pub fn log_response(&mut self, response: &str, duration_ms: u64) -> Result<(), std::io::Error> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            entry_type: LogType::Response,
            round: self.round,
            user_message: None,
            llm_request: None,
            llm_response: Some(response.to_string()),
            tool_calls: None,
            duration_ms: Some(duration_ms),
            error: None,
        };

        self.write_entry(&entry)
    }

    /// Log a tool call
    pub fn log_tool_call(&mut self, tool: &str, args: &Value) -> Result<(), std::io::Error> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            entry_type: LogType::ToolCall,
            round: self.round,
            user_message: None,
            llm_request: None,
            llm_response: None,
            tool_calls: Some(vec![ToolCallInfo {
                tool: tool.to_string(),
                args: args.clone(),
            }]),
            duration_ms: None,
            error: None,
        };

        self.write_entry(&entry)
    }

    /// Log an error
    pub fn log_error(&mut self, error: &str) -> Result<(), std::io::Error> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            entry_type: LogType::Error,
            round: self.round,
            user_message: None,
            llm_request: None,
            llm_response: None,
            tool_calls: None,
            duration_ms: None,
            error: Some(error.to_string()),
        };

        self.write_entry(&entry)
    }

    /// Flush the log buffer to disk
    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        self.log_file.flush()
    }

    /// Write a log entry as JSON
    fn write_entry(&mut self, entry: &LogEntry) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(entry)?;
        writeln!(self.log_file, "{}", json)?;
        self.log_file.flush()
    }
}

/// Log entry structure
#[derive(Debug, Serialize, Deserialize)]
struct LogEntry {
    timestamp: DateTime<Utc>,
    #[serde(rename = "type")]
    entry_type: LogType,
    round: usize,
    user_message: Option<String>,
    llm_request: Option<String>,
    llm_response: Option<String>,
    tool_calls: Option<Vec<ToolCallInfo>>,
    duration_ms: Option<u64>,
    error: Option<String>,
}

/// Type of log entry
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LogType {
    Request,
    Response,
    ToolCall,
    Error,
}

/// Tool call information
#[derive(Debug, Serialize, Deserialize)]
struct ToolCallInfo {
    tool: String,
    args: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            entry_type: LogType::Request,
            round: 1,
            user_message: Some("Hello".to_string()),
            llm_request: None,
            llm_response: None,
            tool_calls: None,
            duration_ms: None,
            error: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"type\":\"request\""));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_logger_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");

        // Change to temp directory for the test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = LlmLogger::new();
        assert!(result.is_ok());
        assert!(log_dir.exists());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}
