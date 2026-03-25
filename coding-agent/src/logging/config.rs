//! Logging configuration for CodingAgent
//!
//! Supports loading configuration from:
//! - Environment variables
//! - Configuration files (config/logging.toml)
//! - Code defaults

use std::path::PathBuf;
use log::LevelFilter;
use serde::{Deserialize, Serialize};

/// File output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOutputConfig {
    /// Path to the log file
    pub path: PathBuf,
    /// Minimum log level to write to file
    pub level: String,
    /// Maximum file size in MB before rotation
    pub max_size_mb: usize,
    /// Number of rotated files to keep
    pub max_files: usize,
}

impl Default for FileOutputConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("logs/coding-agent.log"),
            level: "debug".to_string(),
            max_size_mb: 100,
            max_files: 5,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Default log level (error, warn, info, debug, trace)
    pub default_level: String,
    /// Whether to use colored output
    pub colored: bool,
    /// Whether to show timestamps
    pub show_timestamp: bool,
    /// Whether to show module paths
    pub show_module: bool,
    /// Whether to use emoji indicators
    pub use_emoji: bool,
    /// File output configuration (optional)
    pub file_output: Option<FileOutputConfig>,
    /// Per-module log level overrides
    #[serde(default)]
    pub modules: Vec<ModuleLevelConfig>,
    /// TUI-specific configuration
    #[serde(default)]
    pub tui: TuiLogConfig,
}

/// Per-module log level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLevelConfig {
    /// Module path pattern (e.g., "coding-agent::tools")
    pub module: String,
    /// Log level for this module
    pub level: String,
}

/// TUI-specific logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiLogConfig {
    /// Whether to log to TUI panel
    pub log_to_panel: bool,
    /// Maximum number of log lines to keep
    pub max_log_lines: usize,
    /// Whether to show debug panel by default
    pub show_debug_panel: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            default_level: "info".to_string(),
            colored: true,
            show_timestamp: true,
            show_module: false,
            use_emoji: false,
            file_output: None,
            modules: Vec::new(),
            tui: TuiLogConfig::default(),
        }
    }
}

impl Default for TuiLogConfig {
    fn default() -> Self {
        Self {
            log_to_panel: true,
            max_log_lines: 1000,
            show_debug_panel: false,
        }
    }
}

impl LoggingConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: LoggingConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from environment variables and defaults
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Check for RUST_LOG
        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            config.default_level = rust_log.split(',').next().unwrap_or("info").to_string();
        }

        // Check for colored output
        if let Ok(colored) = std::env::var("CODING_AGENT_LOG_COLORED") {
            config.colored = colored != "false" && colored != "0";
        }

        // Check for timestamp display
        if let Ok(show_timestamp) = std::env::var("CODING_AGENT_LOG_TIMESTAMP") {
            config.show_timestamp = show_timestamp != "false" && show_timestamp != "0";
        }

        // Check for module display
        if let Ok(show_module) = std::env::var("CODING_AGENT_LOG_MODULE") {
            config.show_module = show_module == "true" || show_module == "1";
        }

        // Check for emoji usage
        if let Ok(use_emoji) = std::env::var("CODING_AGENT_LOG_EMOJI") {
            config.use_emoji = use_emoji == "true" || use_emoji == "1";
        }

        // Check for TUI debug panel
        if let Ok(tui_debug) = std::env::var("CODING_AGENT_TUI_DEBUG") {
            config.tui.show_debug_panel = tui_debug == "true" || tui_debug == "1";
        }

        // Check for file logging
        if let Ok(file_path) = std::env::var("CODING_AGENT_LOG_FILE") {
            config.file_output = Some(FileOutputConfig {
                path: PathBuf::from(file_path),
                level: std::env::var("CODING_AGENT_LOG_FILE_LEVEL")
                    .unwrap_or_else(|_| "debug".to_string()),
                max_size_mb: std::env::var("CODING_AGENT_LOG_MAX_SIZE_MB")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100),
                max_files: std::env::var("CODING_AGENT_LOG_MAX_FILES")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5),
            });
        }

        config
    }

    /// Load configuration with fallback chain: file -> env -> defaults
    pub fn load() -> Self {
        // Try to load from config file first
        let config_path = "config/logging.toml";
        let mut config = if std::path::Path::new(config_path).exists() {
            Self::from_file(config_path).unwrap_or_else(|e| {
                eprintln!("Warning: Failed to load logging config from {}: {}", config_path, e);
                Self::default()
            })
        } else {
            Self::default()
        };

        // Override with environment variables
        let env_config = Self::from_env();

        // Environment variables take precedence for certain settings
        if std::env::var("RUST_LOG").is_ok() {
            config.default_level = env_config.default_level;
        }
        if std::env::var("CODING_AGENT_LOG_COLORED").is_ok() {
            config.colored = env_config.colored;
        }
        if std::env::var("CODING_AGENT_TUI_DEBUG").is_ok() {
            config.tui.show_debug_panel = env_config.tui.show_debug_panel;
        }
        if std::env::var("CODING_AGENT_LOG_FILE").is_ok() {
            config.file_output = env_config.file_output;
        }

        config
    }

    /// Parse default level to LevelFilter
    pub fn default_level_filter(&self) -> LevelFilter {
        self.parse_level(&self.default_level)
    }

    /// Parse a level string to LevelFilter
    pub fn parse_level(&self, level: &str) -> LevelFilter {
        match level.to_lowercase().as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            "off" => LevelFilter::Off,
            _ => LevelFilter::Info,
        }
    }

    /// Get log level for a specific module
    pub fn module_level(&self, module_path: &str) -> Option<LevelFilter> {
        for module_config in &self.modules {
            if module_path.starts_with(&module_config.module) {
                return Some(self.parse_level(&module_config.level));
            }
        }
        None
    }

    /// Check if TUI debug panel should be shown by default
    pub fn show_tui_debug_panel(&self) -> bool {
        self.tui.show_debug_panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.default_level, "info");
        assert!(config.colored);
        assert!(config.show_timestamp);
        assert!(!config.show_module);
        assert!(!config.use_emoji);
    }

    #[test]
    fn test_parse_level() {
        let config = LoggingConfig::default();
        assert_eq!(config.parse_level("error"), LevelFilter::Error);
        assert_eq!(config.parse_level("warn"), LevelFilter::Warn);
        assert_eq!(config.parse_level("info"), LevelFilter::Info);
        assert_eq!(config.parse_level("debug"), LevelFilter::Debug);
        assert_eq!(config.parse_level("trace"), LevelFilter::Trace);
        assert_eq!(config.parse_level("invalid"), LevelFilter::Info);
    }

    #[test]
    fn test_module_level() {
        let mut config = LoggingConfig::default();
        config.modules = vec![
            ModuleLevelConfig {
                module: "coding-agent::tools".to_string(),
                level: "debug".to_string(),
            },
            ModuleLevelConfig {
                module: "tirea".to_string(),
                level: "warn".to_string(),
            },
        ];

        assert_eq!(
            config.module_level("coding-agent::tools::bash"),
            Some(LevelFilter::Debug)
        );
        assert_eq!(
            config.module_level("tirea::agent"),
            Some(LevelFilter::Warn)
        );
        assert_eq!(config.module_level("other::module"), None);
    }
}
