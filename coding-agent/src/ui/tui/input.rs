//! Input widget for TUI
//!
//! Handles multi-line text input with @ file reference support.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tui_textarea::{TextArea, CursorMove};

/// Input mode state
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// Normal text input
    Normal,
    /// File selector mode
    FileSelector,
}

/// Input widget state
pub struct InputWidget {
    /// Text area for multi-line input
    textarea: TextArea<'static>,
    /// Current input mode
    mode: InputMode,
    /// Whether to send message
    ready_to_send: bool,
}

impl InputWidget {
    /// Create a new input widget
    pub fn new() -> Self {
        let textarea = TextArea::default();
        Self {
            textarea,
            mode: InputMode::Normal,
            ready_to_send: false,
        }
    }

    /// Get current input mode
    pub fn mode(&self) -> &InputMode {
        &self.mode
    }

    /// Set input mode
    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    /// Check if ready to send
    pub fn is_ready_to_send(&self) -> bool {
        self.ready_to_send
    }

    /// Clear the send flag
    pub fn clear_send_flag(&mut self) {
        self.ready_to_send = false;
    }

    /// Get the current input text
    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Clear the input
    pub fn clear(&mut self) {
        let mut textarea = TextArea::default();
        std::mem::swap(&mut self.textarea, &mut textarea);
    }

    /// Insert a character at cursor position
    pub fn insert_char(&mut self, c: char) {
        // Check for @ symbol to trigger file selector
        if c == '@' && self.text().ends_with('@') {
            self.mode = InputMode::FileSelector;
            return;
        }
        self.textarea.insert_char(c);
    }

    /// Handle key event
    /// Returns true if the message should be sent
    pub fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key_event.code {
            KeyCode::Enter => {
                if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+Enter to send
                    self.ready_to_send = true;
                    return true;
                } else {
                    self.textarea.insert_newline();
                }
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Backspace => {
                self.textarea.delete_char();
            }
            KeyCode::Delete => {
                self.textarea.delete_next_char();
            }
            KeyCode::Up => {
                self.textarea.move_cursor(CursorMove::Up);
            }
            KeyCode::Down => {
                self.textarea.move_cursor(CursorMove::Down);
            }
            KeyCode::Left => {
                self.textarea.move_cursor(CursorMove::Back);
            }
            KeyCode::Right => {
                self.textarea.move_cursor(CursorMove::Forward);
            }
            KeyCode::Home => {
                self.textarea.move_cursor(CursorMove::Head);
            }
            KeyCode::End => {
                self.textarea.move_cursor(CursorMove::End);
            }
            _ => {}
        }

        false
    }

    /// Render the input widget
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = if self.mode == InputMode::FileSelector {
            "Input (File Selector Active) - Ctrl+Enter to Send"
        } else {
            "Input - Ctrl+Enter to Send | ESC to Cancel"
        };

        // Create the block
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan));

        // Render the block first
        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Then render the textarea widget inside the block
        let widget = self.textarea.widget();
        frame.render_widget(widget, inner_area);
    }
}

impl Default for InputWidget {
    fn default() -> Self {
        Self::new()
    }
}
