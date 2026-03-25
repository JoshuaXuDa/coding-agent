//! File path autocomplete popup for TUI
//!
//! Provides file browser functionality with filtering and directory navigation.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::fs;
use std::path::{Path, PathBuf};

/// File information for autocomplete
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Whether this is a directory
    pub is_dir: bool,
}

impl FileInfo {
    /// Get display icon for this file
    pub fn icon(&self) -> &str {
        if self.is_dir {
            "📁"
        } else {
            "📄"
        }
    }
}

/// File autocomplete popup state
pub struct FileAutocomplete {
    /// Current base path being scanned
    base_path: PathBuf,
    /// Current filter string
    filter: String,
    /// Matched suggestions
    suggestions: Vec<FileInfo>,
    /// Currently selected index
    selected_index: usize,
    /// Path prefix already entered (e.g., "src/")
    input_prefix: String,
}

impl FileAutocomplete {
    /// Create a new autocomplete popup
    pub fn new(base_dir: &str) -> Self {
        let base_path = PathBuf::from(base_dir);
        let mut autocomplete = Self {
            base_path,
            filter: String::new(),
            suggestions: Vec::new(),
            selected_index: 0,
            input_prefix: String::new(),
        };
        autocomplete.refresh_suggestions();
        autocomplete
    }

    /// Update the filter string and refresh suggestions
    pub fn update_filter(&mut self, filter: String) {
        self.filter = filter;
        self.refresh_suggestions();
    }

    /// Refresh suggestions based on current path and filter
    fn refresh_suggestions(&mut self) {
        self.suggestions.clear();
        self.selected_index = 0;

        // Add parent directory option if not at root
        if self.base_path != PathBuf::from(".") && self.base_path.has_root() {
            self.suggestions.push(FileInfo {
                name: "..".to_string(),
                path: self.base_path.join(".."),
                is_dir: true,
            });
        }

        // Read directory
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files unless explicitly requested
                if name.starts_with('.') && !self.filter.starts_with('.') {
                    continue;
                }

                let is_dir = path.is_dir();

                // Apply filter
                if self.filter.is_empty() || name.to_lowercase().starts_with(&self.filter.to_lowercase()) {
                    self.suggestions.push(FileInfo { name, path, is_dir });
                }
            }
        }

        // Sort: directories first, then files, both alphabetically
        self.suggestions.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
    }

    /// Move selection to next item
    pub fn next(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.suggestions.len();
        }
    }

    /// Move selection to previous item
    pub fn prev(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.suggestions.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Get currently selected item
    pub fn get_selected(&self) -> Option<&FileInfo> {
        self.suggestions.get(self.selected_index)
    }

    /// Enter the selected directory
    pub fn enter_directory(&mut self) -> Option<String> {
        if let Some(selected) = self.get_selected() {
            if selected.is_dir {
                let dir_name = selected.name.clone();
                self.base_path = selected.path.clone();
                self.input_prefix.push_str(&dir_name);
                self.input_prefix.push('/');
                self.filter.clear();
                self.refresh_suggestions();
                return Some(dir_name);
            }
        }
        None
    }

    /// Go to parent directory
    pub fn parent_directory(&mut self) -> bool {
        if self.base_path != PathBuf::from(".") {
            self.base_path = self.base_path.join("..");
            // Remove the last path component from input_prefix
            if let Some(pos) = self.input_prefix.rfind('/') {
                self.input_prefix = self.input_prefix[..pos + 1].to_string();
            } else {
                self.input_prefix.clear();
            }
            self.filter.clear();
            self.refresh_suggestions();
            true
        } else {
            false
        }
    }

    /// Get the complete selected path
    pub fn get_selected_path(&self) -> Option<String> {
        self.get_selected().map(|info| {
            format!("{}{}", self.input_prefix, info.name)
        })
    }

    /// Check if there are any suggestions
    pub fn is_empty(&self) -> bool {
        self.suggestions.is_empty()
    }

    /// Get the number of suggestions
    pub fn len(&self) -> usize {
        self.suggestions.len()
    }

    /// Render the autocomplete popup
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Ensure the area is valid
        if area.width < 10 || area.height < 3 {
            return;  // Too small to render
        }

        // Always clear the area first to make it visible on top
        frame.render_widget(Clear, area);

        if self.is_empty() {
            // Show empty message
            let empty_msg = vec![Line::from(vec![
                Span::styled("📭", Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled("无匹配文件", Style::default().fg(Color::White)),
            ])];

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title("📁 文件选择")
                .style(Style::default().bg(Color::Black));

            frame.render_widget(
                Paragraph::new(empty_msg)
                    .block(block)
                    .wrap(Wrap { trim: true }),
                area,
            );
            return;
        }

        // Build list items
        let items: Vec<ListItem> = self
            .suggestions
            .iter()
            .enumerate()
            .map(|(i, info)| {
                let is_selected = i == self.selected_index;

                // Build the line with icon and name
                let mut spans = vec![
                    Span::styled(info.icon(), Style::default().fg(if is_selected {
                        Color::LightBlue  // Light blue icon for selected item
                    } else {
                        Color::Yellow
                    })),
                    Span::raw(" "),
                ];

                // Highlight matching part
                if !self.filter.is_empty() && info.name.to_lowercase().starts_with(&self.filter.to_lowercase()) {
                    let filter_len = self.filter.len();
                    let name_lower = info.name.to_lowercase();

                    if name_lower.starts_with(&self.filter.to_lowercase()) {
                        // Matching part
                        spans.push(Span::styled(
                            &info.name[..filter_len],
                            Style::default()
                                .fg(if is_selected { Color::White } else { Color::Green })  // Selected: white text
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));
                        // Rest of the name
                        if info.name.len() > filter_len {
                            spans.push(Span::styled(
                                &info.name[filter_len..],
                                Style::default().fg(if is_selected {
                                    Color::White  // Selected: white text
                                } else {
                                    Color::Gray
                                }),
                            ));
                        }
                    }
                } else {
                    spans.push(Span::styled(
                        info.name.as_str(),
                        Style::default().fg(if is_selected {
                            Color::White  // Selected: white text on blue background
                        } else {
                            Color::Gray
                        }),
                    ));
                }

                // Add directory indicator
                if info.is_dir {
                    spans.push(Span::styled("/", Style::default().fg(Color::Cyan)));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        // Calculate visible area (max 10 items)
        let max_height = items.len().min(10) + 2; // +2 for borders
        let mut display_area = area;
        display_area.height = display_area.height.min(max_height as u16);

        // Create the list
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .style(Style::default().bg(Color::Black))  // Add black background
                    .title(format!(
                        "📁 文件选择 {}/{}",
                        self.selected_index + 1,
                        self.suggestions.len()
                    )),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(20, 40, 80))  // Dark blue RGB for better terminal compatibility
                    .fg(Color::Rgb(255, 255, 255))  // Pure white for contrast
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );

        // Render the list (area already cleared above)
        frame.render_widget(list, display_area);
    }
}
