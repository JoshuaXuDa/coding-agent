//! Custom log formatter for CodingAgent
//!
//! Provides a unified, elegant log format with support for:
//! - Colored output (using termcolor)
//! - Timestamps (RFC3339 or local time)
//! - Module paths
//! - Optional emoji indicators

use std::io::Write;
use log::Level;
use termcolor::{Color, ColorSpec, WriteColor};

/// Timestamp format options
#[derive(Debug, Clone, Copy)]
pub enum TimestampFormat {
    /// ISO 8601 / RFC 3339 format (e.g., "2025-03-25T14:30:45Z")
    RFC3339,
    /// Local time format (e.g., "14:30:45")
    Local,
    /// Unix timestamp (e.g., "1711369845")
    Unix,
}

/// Custom formatter configuration
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Whether to use colored output
    pub colored: bool,
    /// Whether to show timestamps
    pub show_timestamp: bool,
    /// Whether to show module paths
    pub show_module: bool,
    /// Whether to use emoji indicators
    pub use_emoji: bool,
    /// Timestamp format
    pub timestamp_format: TimestampFormat,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            colored: true,
            show_timestamp: true,
            show_module: false,
            use_emoji: false,
            timestamp_format: TimestampFormat::Local,
        }
    }
}

/// Custom log formatter for CodingAgent
pub struct CodingAgentFormatter {
    config: FormatterConfig,
}

impl CodingAgentFormatter {
    /// Create a new formatter with the given configuration
    pub fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// Create a formatter with default configuration
    pub fn default() -> Self {
        Self::new(FormatterConfig::default())
    }

    /// Format a log record and write to the output
    pub fn format<W: WriteColor>(
        &self,
        writer: &mut W,
        level: Level,
        module_path: Option<&str>,
        message: &str,
    ) -> std::io::Result<()> {
        // Reset color
        writer.reset()?;

        // Write timestamp if enabled
        if self.config.show_timestamp {
            self.write_timestamp(writer)?;
            writer.write_all(b" ")?;
        }

        // Write log level with color
        self.write_level(writer, level)?;
        writer.write_all(b" ")?;

        // Write module path if enabled
        if self.config.show_module {
            if let Some(module) = module_path {
                self.write_module(writer, module)?;
                writer.write_all(b": ")?;
            }
        }

        // Write the message
        writer.write_all(message.as_bytes())?;
        writer.write_all(b"\n")?;

        writer.flush()
    }

    /// Write timestamp to output
    fn write_timestamp<W: WriteColor>(&self, writer: &mut W) -> std::io::Result<()> {
        let timestamp = match self.config.timestamp_format {
            TimestampFormat::RFC3339 => {
                format!("[{}]", chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            }
            TimestampFormat::Local => {
                format!("[{}]", chrono::Local::now().format("%H:%M:%S"))
            }
            TimestampFormat::Unix => {
                format!("[{}]", chrono::Utc::now().timestamp())
            }
        };

        let mut color = ColorSpec::new();
        color.set_fg(Some(Color::Rgb(128, 128, 128))); // Dark gray
        writer.set_color(&color)?;
        writer.write_all(timestamp.as_bytes())?;
        writer.reset()?;
        Ok(())
    }

    /// Write log level with appropriate color and optional emoji
    fn write_level<W: WriteColor>(&self, writer: &mut W, level: Level) -> std::io::Result<()> {
        let (level_str, emoji, color) = match level {
            Level::Error => ("ERROR", "❌", Color::Red),
            Level::Warn => ("WARN", "⚠️", Color::Yellow),
            Level::Info => ("INFO", "ℹ️", Color::Green),
            Level::Debug => ("DEBUG", "🔍", Color::Cyan),
            Level::Trace => ("TRACE", "📝", Color::Rgb(128, 128, 128)),
        };

        let mut color_spec = ColorSpec::new();
        if self.config.colored {
            color_spec.set_fg(Some(color));
        }
        writer.set_color(&color_spec)?;

        if self.config.use_emoji {
            write!(writer, "{} ", emoji)?;
        }

        write!(writer, "{:5}", level_str)?;
        writer.reset()?;
        Ok(())
    }

    /// Write module path
    fn write_module<W: WriteColor>(&self, writer: &mut W, module: &str) -> std::io::Result<()> {
        let mut color = ColorSpec::new();
        if self.config.colored {
            color.set_fg(Some(Color::Rgb(100, 149, 237))); // Cornflower blue
        }
        writer.set_color(&color)?;
        writer.write_all(module.as_bytes())?;
        writer.reset()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use termcolor::Ansi;

    #[test]
    fn test_formatter_default_config() {
        let formatter = CodingAgentFormatter::default();
        assert!(formatter.config.colored);
        assert!(formatter.config.show_timestamp);
        assert!(!formatter.config.show_module);
        assert!(!formatter.config.use_emoji);
    }

    #[test]
    fn test_formatter_custom_config() {
        let config = FormatterConfig {
            colored: false,
            show_timestamp: false,
            show_module: true,
            use_emoji: true,
            timestamp_format: TimestampFormat::Unix,
        };
        let formatter = CodingAgentFormatter::new(config.clone());
        assert_eq!(formatter.config.colored, config.colored);
        assert_eq!(formatter.config.show_module, config.show_module);
    }

    #[test]
    fn test_format_output() {
        let formatter = CodingAgentFormatter::default();
        let mut buffer = Ansi::new(Vec::new());

        formatter
            .format(&mut buffer, Level::Info, Some("test_module"), "Test message")
            .unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("INFO"));
        assert!(output.contains("Test message"));
    }
}
