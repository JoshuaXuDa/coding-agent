//! Input status indicator for TUI
//!
//! Provides visual feedback for input state (typing, sending, sent)

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Input status enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputStatus {
    /// Idle state
    Idle,
    /// User is typing
    Typing,
    /// Message is sending
    Sending,
    /// Message was sent
    Sent,
}

/// Input status indicator
pub struct InputStatusIndicator {
    status: InputStatus,
    /// When the status was last updated (for timing)
    status_updated: std::time::Instant,
}

impl InputStatusIndicator {
    /// Create a new status indicator
    pub fn new() -> Self {
        Self {
            status: InputStatus::Idle,
            status_updated: std::time::Instant::now(),
        }
    }

    /// Set the current status
    pub fn set_status(&mut self, status: InputStatus) {
        self.status = status;
        self.status_updated = std::time::Instant::now();
    }

    /// Get the current status
    pub fn status(&self) -> InputStatus {
        self.status
    }

    /// Check if should auto-reset to Idle (for temporary states like Sent)
    pub fn check_auto_reset(&mut self) -> bool {
        const AUTO_RESET_DURATION: std::time::Duration = std::time::Duration::from_secs(1);

        if self.status == InputStatus::Sent
            && self.status_updated.elapsed() > AUTO_RESET_DURATION
        {
            self.status = InputStatus::Idle;
            return true;
        }

        // Also reset Typing to Idle after 5 seconds of no activity
        const TYPING_RESET_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

        if self.status == InputStatus::Typing
            && self.status_updated.elapsed() > TYPING_RESET_DURATION
        {
            self.status = InputStatus::Idle;
            return true;
        }

        false
    }

    /// Get the status text and style for display
    fn get_display(&self) -> (&'static str, &'static str, Color) {
        match self.status {
            InputStatus::Idle => ("", "", Color::Gray),
            InputStatus::Typing => ("✎", "输入中", Color::Cyan),
            InputStatus::Sending => ("⏳", "发送中", Color::Yellow),
            InputStatus::Sent => ("✓", "已发送", Color::Green),
        }
    }

    /// Render the status indicator
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Check for auto-reset before rendering
        self.check_auto_reset();

        let (icon, text, color) = self.get_display();

        // Don't render if idle
        if self.status == InputStatus::Idle {
            return;
        }

        let status_line = Line::from(vec![
            Span::styled(icon, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(text, Style::default().fg(color)),
        ]);

        let paragraph = Paragraph::new(status_line)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}

impl Default for InputStatusIndicator {
    fn default() -> Self {
        Self::new()
    }
}
