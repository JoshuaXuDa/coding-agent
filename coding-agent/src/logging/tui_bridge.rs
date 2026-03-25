//! TUI log bridge for CodingAgent
//!
//! Bridges standard log calls to TUI event system for display in debug panel.

use log::{Level, LevelFilter, Metadata, Record};
use std::sync::Mutex;
use tokio::sync::mpsc;

/// TUI log bridge - forwards log records to TUI event channel
pub struct TuiLogBridge {
    /// Channel sender for TUI events
    sender: mpsc::UnboundedSender<LogEntry>,
    /// Current log level filter
    filter: Mutex<LevelFilter>,
}

/// Log entry for TUI display
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Log level
    pub level: Level,
    /// Module path
    pub module: Option<String>,
    /// Log message
    pub message: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Local>,
}

impl TuiLogBridge {
    /// Create a new TUI log bridge
    pub fn new(sender: mpsc::UnboundedSender<LogEntry>) -> Self {
        Self {
            sender,
            filter: Mutex::new(LevelFilter::Info),
        }
    }

    /// Create a new TUI log bridge with custom level filter
    pub fn with_level(sender: mpsc::UnboundedSender<LogEntry>, level: LevelFilter) -> Self {
        Self {
            sender,
            filter: Mutex::new(level),
        }
    }

    /// Set the log level filter
    pub fn set_level_filter(&self, level: LevelFilter) {
        let mut filter = self.filter.lock().unwrap();
        *filter = level;
    }

    /// Get the current log level filter
    pub fn level_filter(&self) -> LevelFilter {
        *self.filter.lock().unwrap()
    }

    /// Send a log entry to the TUI
    fn send_log(&self, record: &Record) {
        let entry = LogEntry {
            level: record.level(),
            module: record.module_path().map(|s| s.to_string()),
            message: record.args().to_string(),
            timestamp: chrono::Local::now(),
        };

        // Ignore send errors - TUI might not be active
        let _ = self.sender.send(entry);
    }
}

impl log::Log for TuiLogBridge {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let filter = *self.filter.lock().unwrap();
        metadata.level() <= filter
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.send_log(record);
        }
    }

    fn flush(&self) {
        // Nothing to flush for channel-based logging
    }
}

/// Builder for creating a logger that writes to both console and TUI
pub struct DualLogger {
    builder: env_logger::Builder,
    tui: Option<TuiLogBridge>,
}

impl DualLogger {
    /// Create a new dual logger
    pub fn new(builder: env_logger::Builder, tui: Option<TuiLogBridge>) -> Self {
        Self { builder, tui }
    }

    /// Initialize the dual logger as the global logger
    pub fn init(mut self) -> Result<(), log::SetLoggerError> {
        // Use the console logger as the primary logger
        // The TUI bridge will be used separately
        self.builder.try_init()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry {
            level: Level::Info,
            module: Some("test_module".to_string()),
            message: "Test message".to_string(),
            timestamp: chrono::Local::now(),
        };

        assert_eq!(entry.level, Level::Info);
        assert_eq!(entry.module, Some("test_module".to_string()));
        assert_eq!(entry.message, "Test message");
    }

    #[test]
    fn test_tui_log_bridge_level_filter() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let bridge = TuiLogBridge::with_level(tx, LevelFilter::Debug);

        assert_eq!(bridge.level_filter(), LevelFilter::Debug);
        bridge.set_level_filter(LevelFilter::Info);
        assert_eq!(bridge.level_filter(), LevelFilter::Info);
    }
}
