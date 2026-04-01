//! Permission prompt widget for TUI
//!
//! Displays a modal overlay asking the user to confirm or deny
//! a tool execution request.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// The user's response to a permission prompt.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionResponse {
    AllowOnce,
    AlwaysAllow,
    Deny,
}

/// State for the permission prompt overlay.
pub struct PermissionPrompt {
    pub tool_name: String,
    pub description: String,
    pub active: bool,
}

impl PermissionPrompt {
    /// Create a new inactive permission prompt.
    pub fn new() -> Self {
        Self {
            tool_name: String::new(),
            description: String::new(),
            active: false,
        }
    }

    /// Show the permission prompt for a tool call.
    pub fn show(&mut self, tool_name: String, description: String) {
        self.tool_name = tool_name;
        self.description = description;
        self.active = true;
    }

    /// Dismiss the permission prompt.
    pub fn dismiss(&mut self) {
        self.active = false;
    }

    /// Render the permission prompt overlay.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        // Clear the area behind the prompt
        frame.render_widget(Clear, area);

        // Calculate prompt size
        let width = 60.min(area.width.saturating_sub(4)) as u16;
        let height = 10.min(area.height.saturating_sub(4)) as u16;
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;

        let prompt_area = Rect::new(x, y, width, height);

        let lines = vec![
            Line::from(vec![
                Span::styled("Permission Required", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tool: ", Style::default().fg(Color::Gray)),
                Span::styled(&self.tool_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(&self.description, Style::default().fg(Color::Gray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Y] Allow   [A] Always Allow   [N] Deny", Style::default().fg(Color::Cyan)),
            ]),
        ];

        let paragraph = Paragraph::new(lines)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title("Security"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, prompt_area);
    }
}

impl Default for PermissionPrompt {
    fn default() -> Self {
        Self::new()
    }
}
