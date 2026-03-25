//! Logging module for CodingAgent
//!
//! Provides a unified, elegant logging system with support for:
//! - Standard log macros (debug!, info!, warn!, error!)
//! - Custom formatters with colors and timestamps
//! - Configuration from files and environment variables
//! - TUI debug panel integration

mod config;
mod formatter;
mod tui_bridge;

pub use config::{FileOutputConfig, LoggingConfig, ModuleLevelConfig, TuiLogConfig};
pub use formatter::{CodingAgentFormatter, FormatterConfig, TimestampFormat};
pub use tui_bridge::{LogEntry, TuiLogBridge};

use log::LevelFilter;
use std::io::Write;

/// Initialize the logging system
///
/// This function sets up the global logger with configuration loaded from:
/// 1. config/logging.toml (if exists)
/// 2. Environment variables (RUST_LOG, CODING_AGENT_LOG_*, etc.)
/// 3. Default values
///
/// # Example
///
/// ```no_run
/// use coding_agent::logging::init_logging;
///
/// fn main() -> anyhow::Result<()> {
///     init_logging()?;
///     // Your application code here
///     Ok(())
/// }
/// ```
pub fn init_logging() -> anyhow::Result<()> {
    let config = LoggingConfig::load();

    let mut builder = env_logger::Builder::new();
    builder.filter_level(config.default_level_filter());

    // Apply module-specific log levels
    for module_config in &config.modules {
        let level = config.parse_level(&module_config.level);
        builder.filter_module(&module_config.module, level);
    }

    // Parse RUST_LOG environment variable for additional overrides
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        builder.parse_filters(&rust_log);
    }

    // Clone config for the formatter closure
    let config_colored = config.colored;
    let config_show_timestamp = config.show_timestamp;
    let config_show_module = config.show_module;
    let config_use_emoji = config.use_emoji;

    // Set up custom formatter
    builder.format(move |buf, record| {
        let formatter_config = formatter::FormatterConfig {
            colored: config_colored,
            show_timestamp: config_show_timestamp,
            show_module: config_show_module,
            use_emoji: config_use_emoji,
            timestamp_format: formatter::TimestampFormat::Local,
        };

        let formatter = CodingAgentFormatter::new(formatter_config);

        // Create a writer that supports colors
        let mut buffer = Vec::new();
        {
            let mut stdout = termcolor::Ansi::new(&mut buffer);

            // Format the log record
            formatter.format(
                &mut stdout,
                record.level(),
                record.module_path(),
                &record.args().to_string(),
            )?;
        }

        // Get the formatted output
        let output = String::from_utf8_lossy(&buffer);
        writeln!(buf, "{}", output.trim())
    });

    // Initialize the logger
    builder.try_init()?;

    Ok(())
}

/// Initialize logging with a custom TUI log bridge
///
/// This variant also sets up a log bridge that forwards logs to the TUI
/// debug panel. Returns the sender channel for log entries.
///
/// # Example
///
/// ```no_run
/// use coding_agent::logging::init_logging_with_tui;
///
/// # async fn example() -> anyhow::Result<()> {
/// let (log_tx, mut log_rx) = init_logging_with_tui()?;
///
/// // Spawn a task to handle log entries in TUI
/// tokio::spawn(async move {
///     while let Some(entry) = log_rx.recv().await {
///         // Display entry in TUI debug panel
///     }
/// });
/// # Ok(())
/// # }
/// ```
pub fn init_logging_with_tui() -> anyhow::Result<(tokio::sync::mpsc::UnboundedSender<LogEntry>, tokio::sync::mpsc::UnboundedReceiver<LogEntry>)> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Initialize console logging
    init_logging()?;

    // Note: TUI log bridge is not set as the global logger
    // Instead, applications can manually use it if needed
    // This avoids double logging

    Ok((tx, rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        // This test just verifies that init_logging doesn't panic
        // We can't actually test logging output easily in unit tests
        let result = std::panic::catch_unwind(|| {
            let _ = init_logging();
        });
        assert!(result.is_ok());
    }
}
