//! TUI-based file selector

use crate::platform::domain::filesystem::FileSystem;
use glob::glob;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

/// TUI file selector
pub struct TuiFileSelector {
    fs: Arc<dyn FileSystem>,
    files: Vec<PathBuf>,
    filtered_indices: Vec<usize>,
    selected_idx: usize,
    filter: String,
}

impl TuiFileSelector {
    /// Create a new TUI file selector
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self {
            fs,
            files: Vec::new(),
            filtered_indices: Vec::new(),
            selected_idx: 0,
            filter: String::new(),
        }
    }

    /// Search for files and prepare the selector
    pub fn search(&mut self, pattern: &str) -> io::Result<()> {
        self.files = self.find_files(pattern);
        self.filtered_indices = (0..self.files.len()).collect();
        self.selected_idx = 0;
        Ok(())
    }

    /// Find files matching the pattern
    fn find_files(&self, pattern: &str) -> Vec<PathBuf> {
        let mut matches = Vec::new();

        let search_pattern = if pattern.is_empty() {
            "*".to_string()
        } else if pattern.contains('/') {
            format!("**/*{}*", pattern)
        } else {
            format!("**/*{}*", pattern)
        };

        if let Ok(entries) = glob(&search_pattern) {
            for entry in entries.flatten() {
                if self.fs.is_file(&entry) {
                    matches.push(entry);
                }
            }
        }

        matches.sort();
        matches.truncate(100); // Limit for performance
        matches
    }

    /// Run the TUI selector and return selected file
    pub fn run(&mut self) -> io::Result<Option<PathBuf>> {
        if self.files.is_empty() {
            return Ok(None);
        }

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run UI loop
        let result = self.run_ui(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Main UI loop
    fn run_ui(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<Option<PathBuf>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            // Handle events
            if let event::Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        if !self.filtered_indices.is_empty() {
                            let idx = self.filtered_indices[self.selected_idx];
                            return Ok(Some(self.files[idx].clone()));
                        }
                    }
                    KeyCode::Down => {
                        if !self.filtered_indices.is_empty() {
                            self.selected_idx = (self.selected_idx + 1).min(self.filtered_indices.len() - 1);
                        }
                    }
                    KeyCode::Up => {
                        self.selected_idx = self.selected_idx.saturating_sub(1);
                    }
                    KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                        self.filter.push(c);
                        self.apply_filter();
                        self.selected_idx = 0;
                    }
                    KeyCode::Backspace => {
                        self.filter.pop();
                        self.apply_filter();
                        self.selected_idx = 0;
                    }
                    KeyCode::Home => {
                        self.selected_idx = 0;
                    }
                    KeyCode::End => {
                        self.selected_idx = self.filtered_indices.len().saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Apply filter to files
    fn apply_filter(&mut self) {
        if self.filter.is_empty() {
            self.filtered_indices = (0..self.files.len()).collect();
        } else {
            let lower_filter = self.filter.to_lowercase();
            self.filtered_indices = self.files
                .iter()
                .enumerate()
                .filter(|(_, path)| {
                    path.to_string_lossy()
                        .to_lowercase()
                        .contains(&lower_filter)
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// Draw the UI
    fn ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)].as_ref())
            .split(f.size());

        // Title/search box
        let title = vec![
            Line::from(vec![
                Span::styled("文件选择器", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - "),
                Span::styled("↑↓", Style::default().fg(Color::Green)),
                Span::raw(" 导游 "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" 选择 "),
                Span::styled("q/Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" 取消"),
            ]),
        ];

        let title_widget = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(title_widget, chunks[0]);

        // File list
        let items: Vec<ListItem> = self.filtered_indices
            .iter()
            .enumerate()
            .map(|(list_idx, &file_idx)| {
                let path = &self.files[file_idx];
                let is_selected = list_idx == self.selected_idx;

                let style = if is_selected {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let display_name = if let Some(name) = path.file_name() {
                    name.to_string_lossy().to_string()
                } else {
                    path.display().to_string()
                };

                ListItem::new(display_name).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!("文件 ({})", self.filtered_indices.len())));

        f.render_widget(list, chunks[1]);

        // Filter hint
        let filter_text = if self.filter.is_empty() {
            "输入字符过滤...".to_string()
        } else {
            format!("过滤: {}", self.filter)
        };

        let filter_widget = Paragraph::new(Line::from(vec![
            Span::styled(filter_text, Style::default().fg(Color::Yellow)),
        ]))
        .wrap(Wrap { trim: true });

        f.render_widget(filter_widget, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_selector_creation() {
        // Verify type exists
        let _: TuiFileSelector;
    }
}
