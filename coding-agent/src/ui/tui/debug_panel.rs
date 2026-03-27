//! Debug panel widget for TUI
//!
//! Displays log entries in a dedicated panel within the TUI.

use crate::logging::LogEntry;
use log::{Level, LevelFilter};
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};
use std::collections::VecDeque;

/// Debug panel for displaying log entries
pub struct DebugPanel {
    /// Log entries
    logs: VecDeque<LogEntry>,
    /// Maximum number of log entries to keep
    max_entries: usize,
    /// Current log level filter
    level_filter: LevelFilter,
    /// Scroll offset
    scroll_offset: usize,
    /// Auto-scroll to bottom
    auto_scroll: bool,
}

impl DebugPanel {
    /// Create a new debug panel
    pub fn new(max_entries: usize) -> Self {
        Self {
            logs: VecDeque::with_capacity(max_entries),
            max_entries,
            level_filter: LevelFilter::Info,
            scroll_offset: 0,
            auto_scroll: true,
        }
    }

    /// Create a debug panel with default capacity
    pub fn default() -> Self {
        Self::new(1000)
    }

    /// Add a log entry to the panel
    pub fn add_log(&mut self, entry: LogEntry) {
        // Remove oldest entry if at capacity
        if self.logs.len() >= self.max_entries {
            self.logs.pop_front();
            // Adjust scroll offset if needed
            if self.scroll_offset > 0 {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
        }

        self.logs.push_back(entry);

        // Auto-scroll to bottom if enabled
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Clear all log entries
    pub fn clear(&mut self) {
        self.logs.clear();
        self.scroll_offset = 0;
    }

    /// Set the log level filter
    pub fn set_level_filter(&mut self, level: LevelFilter) {
        self.level_filter = level;
    }

    /// Get the current log level filter
    pub fn level_filter(&self) -> LevelFilter {
        self.level_filter
    }

    /// Cycle through log level filters
    pub fn cycle_level_filter(&mut self) {
        self.level_filter = match self.level_filter {
            LevelFilter::Off => LevelFilter::Error,
            LevelFilter::Error => LevelFilter::Warn,
            LevelFilter::Warn => LevelFilter::Info,
            LevelFilter::Info => LevelFilter::Debug,
            LevelFilter::Trace => LevelFilter::Off,
            LevelFilter::Debug => LevelFilter::Trace,
        };
    }

    /// Get the name of the current level filter
    pub fn level_filter_name(&self) -> &'static str {
        match self.level_filter {
            LevelFilter::Off => "OFF",
            LevelFilter::Error => "ERROR",
            LevelFilter::Warn => "WARN",
            LevelFilter::Info => "INFO",
            LevelFilter::Debug => "DEBUG",
            LevelFilter::Trace => "TRACE",
        }
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
            self.auto_scroll = false;
        }
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self) {
        let visible_count = self.visible_log_count();
        let max_offset = self.logs.len().saturating_sub(visible_count);
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        }
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.logs.len().saturating_sub(self.visible_log_count());
        self.auto_scroll = true;
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
        self.auto_scroll = false;
    }

    /// Page down
    pub fn page_down(&mut self) {
        let visible_count = self.visible_log_count();
        let max_offset = self.logs.len().saturating_sub(visible_count);
        self.scroll_offset = (self.scroll_offset + 10).min(max_offset);
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Get total filtered log count
    pub fn filtered_count(&self) -> usize {
        self.logs.iter().filter(|e| e.level <= self.level_filter).count()
    }

    /// Get the number of visible log entries
    fn visible_log_count(&self) -> usize {
        // Approximate - will be calculated based on actual area during render
        10
    }

    /// Check if panel is empty
    pub fn is_empty(&self) -> bool {
        self.filtered_logs().is_empty()
    }

    /// Get filtered logs based on current level filter
    fn filtered_logs(&self) -> Vec<&LogEntry> {
        self.logs
            .iter()
            .filter(|entry| entry.level <= self.level_filter)
            .collect()
    }

    /// Render the debug panel
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Filter logs based on level
        let filtered_logs: Vec<_> = self
            .logs
            .iter()
            .filter(|entry| entry.level <= self.level_filter)
            .collect();

        // Calculate visible lines
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        let log_count = filtered_logs.len();

        // Calculate scroll offset
        let max_offset = log_count.saturating_sub(visible_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }

        // Get visible logs
        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + visible_height).min(log_count);
        let visible_logs = &filtered_logs[start_idx..end_idx];

        // Build title with level filter
        let title = format!(
            "Debug Panel [Level: {}] [{} logs] [F12:Close l:Level c:Clear]",
            self.level_filter_name(),
            log_count
        );

        // Build log lines
        let lines: Vec<Line> = visible_logs
            .iter()
            .map(|entry| self.format_log_entry(entry))
            .collect();

        // Create the widget
        let block = Block::default()
            .title(title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        // Render logs
        let paragraph = ratatui::widgets::Paragraph::new(lines)
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Format a log entry for display
    fn format_log_entry(&self, entry: &LogEntry) -> Line {
        let timestamp = entry.timestamp.format("%H:%M:%S%.3f").to_string();
        let level_str = format!("{:5}", entry.level.to_string());
        let level_color = self.level_color(entry.level);

        let mut spans = vec![
            Span::styled("[", Style::default().fg(Color::DarkGray)),
            Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(level_str, Style::default().fg(level_color)),
        ];

        if let Some(module) = &entry.module {
            spans.push(Span::styled(
                format!(" [{}] ", module),
                Style::default().fg(Color::Rgb(100, 149, 237)),
            ));
        } else {
            spans.push(Span::raw(" "));
        }

        spans.push(Span::styled(entry.message.clone(), Style::default()));

        Line::from(spans)
    }

    /// Get color for a log level
    fn level_color(&self, level: Level) -> Color {
        match level {
            Level::Error => Color::Red,
            Level::Warn => Color::Yellow,
            Level::Info => Color::Green,
            Level::Debug => Color::Cyan,
            Level::Trace => Color::Rgb(128, 128, 128),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_debug_panel_creation() {
        let panel = DebugPanel::new(100);
        assert_eq!(panel.max_entries, 100);
        assert!(panel.is_empty());
        assert_eq!(panel.level_filter(), LevelFilter::Info);
    }

    #[test]
    fn test_add_log() {
        let mut panel = DebugPanel::new(10);
        let entry = LogEntry {
            level: Level::Info,
            module: Some("test".to_string()),
            message: "Test message".to_string(),
            timestamp: Local::now(),
        };

        panel.add_log(entry);
        assert!(!panel.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut panel = DebugPanel::new(10);
        let entry = LogEntry {
            level: Level::Info,
            module: None,
            message: "Test".to_string(),
            timestamp: Local::now(),
        };

        panel.add_log(entry);
        panel.clear();
        assert!(panel.is_empty());
    }

    #[test]
    fn test_cycle_level_filter() {
        let mut panel = DebugPanel::new(10);
        assert_eq!(panel.level_filter(), LevelFilter::Info);

        panel.cycle_level_filter();
        assert_eq!(panel.level_filter(), LevelFilter::Debug);

        panel.cycle_level_filter();
        assert_eq!(panel.level_filter(), LevelFilter::Trace);

        panel.cycle_level_filter();
        assert_eq!(panel.level_filter(), LevelFilter::Off);
    }

    #[test]
    fn test_max_entries_limit() {
        let mut panel = DebugPanel::new(5);

        for i in 0..10 {
            let entry = LogEntry {
                level: Level::Info,
                module: None,
                message: format!("Message {}", i),
                timestamp: Local::now(),
            };
            panel.add_log(entry);
        }

        // Should only keep the last 5 entries
        assert_eq!(panel.logs.len(), 5);
    }
}
