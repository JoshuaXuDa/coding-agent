//! Input widget for TUI
//!
//! Handles multi-line text input with @ file reference support.

use crate::ui::tui::autocomplete::FileAutocomplete;
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
    /// Autocomplete popup mode
    Autocomplete,
}

/// Input widget state
pub struct InputWidget {
    /// Text area for multi-line input
    textarea: TextArea<'static>,
    /// Current input mode
    mode: InputMode,
    /// Whether to send message
    ready_to_send: bool,
    /// Autocomplete popup state
    pub autocomplete: Option<FileAutocomplete>,
    /// Track the @ symbol position for autocomplete
    pub autocomplete_trigger_pos: Option<usize>,
}

impl InputWidget {
    /// Create a new input widget
    pub fn new() -> Self {
        let textarea = TextArea::default();
        Self {
            textarea,
            mode: InputMode::Normal,
            ready_to_send: false,
            autocomplete: None,
            autocomplete_trigger_pos: None,
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
        // Check for @ symbol to trigger autocomplete
        if c == '@' {
            let current_text = self.text();
            // Only trigger if @ is not already at the end (avoid double trigger)
            if !current_text.ends_with('@') {
                self.mode = InputMode::Autocomplete;
                self.autocomplete = Some(FileAutocomplete::new("."));
                self.autocomplete_trigger_pos = Some(current_text.len());
                self.textarea.insert_char(c);
                return;
            }
        }

        // If in autocomplete mode, handle filtering
        if self.mode == InputMode::Autocomplete {
            // Build current filter from text after @
            let current_text = self.text();
            if let Some(trigger_pos) = self.autocomplete_trigger_pos {
                let filter: String = current_text[trigger_pos + 1..].chars().collect();
                if let Some(autocomplete) = &mut self.autocomplete {
                    autocomplete.update_filter(filter);
                }
            }
        }

        self.textarea.insert_char(c);
    }

    /// Handle key event
    /// Returns true if the message should be sent
    pub fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Handle autocomplete mode specially
        if self.mode == InputMode::Autocomplete {
            return self.handle_autocomplete_key_event(key_event);
        }

        match key_event.code {
            KeyCode::Enter => {
                if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                    // Shift+Enter for newline
                    self.textarea.insert_newline();
                } else {
                    // Enter to send (mainstream UX)
                    self.ready_to_send = true;
                    return true;
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

    /// Handle key events in autocomplete mode
    /// Returns true if the message should be sent
    fn handle_autocomplete_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key_event.code {
            KeyCode::Up => {
                if let Some(autocomplete) = &mut self.autocomplete {
                    autocomplete.prev();
                }
            }
            KeyCode::Down => {
                if let Some(autocomplete) = &mut self.autocomplete {
                    autocomplete.next();
                }
            }
            KeyCode::Enter => {
                if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                    // Shift+Enter for newline in autocomplete mode
                    self.textarea.insert_newline();
                } else if let Some(autocomplete) = &mut self.autocomplete {
                    // Try to enter directory or select file
                    if let Some(dir_name) = autocomplete.enter_directory() {
                        // Directory entered - update textarea with dir name
                        self.textarea.insert_str(&format!("{}/", dir_name));
                    } else {
                        // File selected - insert full path and exit autocomplete
                        if let Some(path) = autocomplete.get_selected_path() {
                            // Remove the @ and incomplete path, then insert the full path
                            if let Some(trigger_pos) = self.autocomplete_trigger_pos {
                                // Clear everything from @ onwards
                                let current_text = self.text();
                                let before_at = &current_text[..trigger_pos];
                                self.clear();
                                for c in before_at.chars() {
                                    self.textarea.insert_char(c);
                                }
                                // Insert the full path
                                self.textarea.insert_str(&path);
                            }
                        }
                        // Exit autocomplete mode
                        self.exit_autocomplete();
                    }
                }
            }
            KeyCode::Char(c) => {
                // Check if we're still in autocomplete context
                if let Some(trigger_pos) = self.autocomplete_trigger_pos {
                    let current_text = self.text();
                    // If user typed space or other non-path character, exit autocomplete
                    if c == ' ' {
                        self.textarea.insert_char(c);
                        self.exit_autocomplete();
                    } else {
                        // Otherwise insert character and update filter
                        self.textarea.insert_char(c);
                        if let Some(autocomplete) = &mut self.autocomplete {
                            let filter: String = current_text[trigger_pos + 1..].chars().chain(Some(c)).collect();
                            autocomplete.update_filter(filter);
                        }
                    }
                } else {
                    self.textarea.insert_char(c);
                    self.exit_autocomplete();
                }
            }
            KeyCode::Backspace => {
                // Check if we're about to delete the @ or go back to parent directory
                let current_text = self.text();
                if let Some(trigger_pos) = self.autocomplete_trigger_pos {
                    if current_text.len() <= trigger_pos + 1 {
                        // Would delete the @, so exit autocomplete
                        self.textarea.delete_char();
                        self.exit_autocomplete();
                    } else {
                        // Check if we're deleting back to a / (parent directory)
                        let text_after_at = &current_text[trigger_pos + 1..];
                        if text_after_at.ends_with('/') && text_after_at.len() > 1 {
                            // Try to go to parent directory
                            if let Some(autocomplete) = &mut self.autocomplete {
                                if autocomplete.parent_directory() {
                                    // Delete the / and dir name
                                    for _ in 0..=text_after_at.trim_end_matches('/').chars().count() {
                                        self.textarea.delete_char();
                                    }
                                }
                            }
                        } else {
                            // Normal delete and update filter
                            self.textarea.delete_char();
                            let new_text = self.text();
                            if let Some(autocomplete) = &mut self.autocomplete {
                                if new_text.len() > trigger_pos + 1 {
                                    let filter: String = new_text[trigger_pos + 1..].chars().collect();
                                    autocomplete.update_filter(filter);
                                } else {
                                    autocomplete.update_filter(String::new());
                                }
                            }
                        }
                    }
                } else {
                    self.textarea.delete_char();
                    self.exit_autocomplete();
                }
            }
            _ => {
                // For other keys, exit autocomplete mode and handle normally
                self.exit_autocomplete();
                // Re-handle this key event in normal mode
                return self.handle_key_event(key_event);
            }
        }

        false
    }

    /// Exit autocomplete mode
    fn exit_autocomplete(&mut self) {
        self.mode = InputMode::Normal;
        self.autocomplete = None;
        self.autocomplete_trigger_pos = None;
    }

    /// Check if autocomplete is active
    pub fn is_autocomplete_active(&self) -> bool {
        self.mode == InputMode::Autocomplete && self.autocomplete.is_some()
    }

    /// Render the input widget
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let title = if self.mode == InputMode::Autocomplete {
            "📁 文件选择 | ↑↓:选择 Enter:确认 Esc:取消"
        } else {
            "💬 输入消息 | @:文件选择 Enter:发送 Shift+Enter:换行 | ESC:退出"
        };

        // 根据输入状态改变边框颜色
        let border_color = if self.mode == InputMode::Autocomplete {
            Color::Yellow  // 自动补全模式显示黄色边框
        } else if self.text().trim().is_empty() {
            Color::Gray
        } else {
            Color::Cyan  // 有输入时显示青色边框
        };

        // Create the block
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

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
