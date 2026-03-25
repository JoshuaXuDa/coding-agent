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
use walkdir::WalkDir;

/// File information for autocomplete
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Relative path from root (e.g., "src/lib.rs")
    pub relative_path: String,
    /// Depth level (0 = root, 1 = immediate subdirectory, etc.)
    pub depth: usize,
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
    /// Maximum search depth
    max_depth: usize,
    /// Maximum number of results
    max_results: usize,
    /// Root path for relative path calculation
    root_path: PathBuf,
}

impl FileAutocomplete {
    /// Create a new autocomplete popup
    pub fn new(base_dir: &str) -> Self {
        let base_path = PathBuf::from(base_dir);
        let root_path = base_path.clone();
        let mut autocomplete = Self {
            base_path,
            filter: String::new(),
            suggestions: Vec::new(),
            selected_index: 0,
            input_prefix: String::new(),
            max_depth: 4,
            max_results: 100,
            root_path,
        };
        autocomplete.refresh_suggestions();
        autocomplete
    }

    /// Update the filter string and refresh suggestions
    pub fn update_filter(&mut self, filter: String) {
        self.filter = filter;
        self.refresh_suggestions();
    }

    /// Scan directory recursively with depth and result limits
    fn scan_recursive(&mut self) {
        self.suggestions.clear();
        self.selected_index = 0;

        // Add parent directory option if not at root
        if self.base_path != self.root_path && self.base_path.has_root() {
            self.suggestions.push(FileInfo {
                name: "..".to_string(),
                path: self.base_path.join(".."),
                is_dir: true,
                relative_path: "..".to_string(),
                depth: 0,
            });
        }

        // Directories to ignore (common large directories)
        let ignored_dirs = [
            "node_modules", "target", ".git", "vendor",
            ".cargo", ".idea", ".vscode", "dist", "build",
            "out", "__pycache__", ".pytest_cache",
        ];

        // If filter is empty, use simple directory read
        if self.filter.is_empty() {
            self.scan_current_directory(&ignored_dirs);
        } else {
            self.scan_recursive_with_filter(&ignored_dirs);
        }
    }

    /// Scan only the current directory (no recursion)
    fn scan_current_directory(&mut self, ignored_dirs: &[&str]) {
        let root_path = &self.root_path;

        // Read current directory
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }

                // Check if it's a directory
                let is_dir = entry.file_type().ok().map(|t| t.is_dir()).unwrap_or(false);

                // Skip ignored directories
                if is_dir {
                    let name_lower = name.to_lowercase();
                    if ignored_dirs.contains(&name_lower.as_ref()) {
                        continue;
                    }
                }

                let path = entry.path();
                let relative_path = path
                    .strip_prefix(root_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                self.suggestions.push(FileInfo {
                    name: name.clone(),
                    path,
                    is_dir,
                    relative_path,
                    depth: 1,
                });
            }
        }

        // Sort: directories first, then alphabetically
        self.suggestions.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
    }

    /// Scan recursively with filter
    fn scan_recursive_with_filter(&mut self, ignored_dirs: &[&str]) {
        let root_path = &self.root_path;
        let mut results = Vec::new();

        // Walk directory with depth limit
        let walker = WalkDir::new(&self.base_path)
            .min_depth(1)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden files unless explicitly requested
                let name = e.file_name().to_string_lossy();
                if name.starts_with('.') && !self.filter.starts_with('.') {
                    return false;
                }

                // Skip known large directories
                if e.file_type().is_dir() {
                    let name_lower = name.to_lowercase();
                    !ignored_dirs.contains(&name_lower.as_ref())
                } else {
                    true
                }
            })
            .filter_map(|e| e.ok());

        for entry in walker {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().is_dir();
            let depth = entry.depth();

            // Calculate relative path
            let relative_path = path
                .strip_prefix(root_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            // Apply filter to relative path (not just name)
            let filter_lower = self.filter.to_lowercase();
            let relative_path_lower = relative_path.to_lowercase();

            if relative_path_lower.contains(&filter_lower) {
                results.push(FileInfo {
                    name: name.clone(),
                    path: path.to_path_buf(),
                    is_dir,
                    relative_path,
                    depth,
                });

                // Stop if we've hit the limit
                if results.len() >= self.max_results {
                    break;
                }
            }
        }

        self.suggestions = results;

        // Sort: directories first, then by depth, then alphabetically
        self.suggestions.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    a.depth.cmp(&b.depth)
                        .then_with(|| a.relative_path.cmp(&b.relative_path))
                }
            }
        });
    }

    /// Refresh suggestions based on current path and filter
    fn refresh_suggestions(&mut self) {
        // Always use recursive search
        self.scan_recursive();
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
                // Clone needed values to release the borrow
                let full_path = selected.path.clone();
                let relative_path = selected.relative_path.clone();
                let name = selected.name.clone();

                // Update base_path to the selected directory
                self.base_path = full_path;

                // Update input_prefix with relative path
                self.input_prefix.push_str(&relative_path);
                self.input_prefix.push('/');

                self.filter.clear();
                self.refresh_suggestions();
                return Some(name);
            }
        }
        None
    }

    /// Go to parent directory
    pub fn parent_directory(&mut self) -> bool {
        if self.base_path != self.root_path {
            self.base_path = self.base_path.join("..");
            // Normalize the path
            self.base_path = self.base_path.canonicalize().unwrap_or_else(|_| {
                // If canonicalize fails (e.g., directory doesn't exist), clean the path
                path_clean::PathClean::clean(&self.base_path)
            });

            // Remove the last path component from input_prefix
            if let Some(pos) = self.input_prefix.rfind('/') {
                // Ensure the slice is within bounds
                if pos + 1 <= self.input_prefix.len() {
                    self.input_prefix = self.input_prefix[..pos + 1].to_string();
                } else {
                    self.input_prefix.clear();
                }
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
            // Return relative path directly
            info.relative_path.clone()
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

                // Build display name - only use depth indicator when filtering
                let display_name = if self.filter.is_empty() {
                    // When filter is empty, show just the name (current directory)
                    info.name.clone()
                } else {
                    // When filtering, show relative path with depth indicator
                    if info.depth > 0 {
                        format!("├─ {}", info.relative_path)
                    } else {
                        info.relative_path.clone()
                    }
                };

                // Truncate long paths (using character-based slicing for UTF-8 safety)
                let display_name = if display_name.chars().count() > 40 {
                    let chars: Vec<char> = display_name.chars().collect();
                    let truncate_len = 37.min(chars.len());
                    let truncated: String = chars[..truncate_len].iter().collect();
                    format!("{}...", truncated)
                } else {
                    display_name
                };

                let mut spans = vec![
                    Span::styled(info.icon(), Style::default().fg(if is_selected {
                        Color::LightBlue
                    } else {
                        Color::Yellow
                    })),
                    Span::raw(" "),
                ];

                // Highlight matching part
                if !self.filter.is_empty() {
                    let filter_lower = self.filter.to_lowercase();
                    let text_to_search = if self.filter.is_empty() {
                        &info.name
                    } else {
                        &info.relative_path
                    };
                    let text_to_search_lower = text_to_search.to_lowercase();

                    if let Some(byte_pos) = text_to_search_lower.find(&filter_lower) {
                        // Convert byte position to character position for UTF-8 safety
                        let char_pos = text_to_search_lower[..byte_pos].chars().count();
                        let filter_char_len = self.filter.chars().count();
                        let display_name_chars: Vec<char> = display_name.chars().collect();
                        let total_chars = display_name_chars.len();

                        // Before match
                        if char_pos > 0 && char_pos <= total_chars {
                            let before: String = display_name_chars[..char_pos].iter().collect();
                            spans.push(Span::styled(
                                before,
                                Style::default().fg(if is_selected {
                                    Color::White
                                } else {
                                    Color::Gray
                                }),
                            ));
                        }

                        // Matching part
                        let match_end = (char_pos + filter_char_len).min(total_chars);
                        if char_pos < match_end && char_pos <= total_chars && match_end <= total_chars {
                            let matched: String = display_name_chars[char_pos..match_end].iter().collect();
                            spans.push(Span::styled(
                                matched,
                                Style::default()
                                    .fg(if is_selected { Color::White } else { Color::Green })
                                    .add_modifier(ratatui::style::Modifier::BOLD),
                            ));
                        }

                        // After match
                        if match_end < total_chars && match_end <= total_chars {
                            let after: String = display_name_chars[match_end..].iter().collect();
                            spans.push(Span::styled(
                                after,
                                Style::default().fg(if is_selected {
                                    Color::White
                                } else {
                                    Color::Gray
                                }),
                            ));
                        }
                    } else {
                        // No match found (shouldn't happen if filter worked)
                        spans.push(Span::styled(
                            display_name.clone(),
                            Style::default().fg(if is_selected {
                                Color::White
                            } else {
                                Color::Gray
                            }),
                        ));
                    }
                } else {
                    spans.push(Span::styled(
                        display_name.clone(),
                        Style::default().fg(if is_selected {
                            Color::White
                        } else {
                            Color::Gray
                        }),
                    ));
                }

                // Add directory indicator
                if info.is_dir {
                    spans.push(Span::styled("/".to_string(), Style::default().fg(Color::Cyan)));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursive_search() {
        // Test with a temporary directory structure
        let autocomplete = FileAutocomplete::new("/tmp/test_autocomplete");

        // Should find files in subdirectories
        assert!(!autocomplete.is_empty(), "Should find files");

        // Check that suggestions include files from subdirectories
        let has_subdir_file = autocomplete.suggestions.iter().any(|f| f.relative_path.contains("src/") || f.relative_path.contains("tests/"));
        assert!(has_subdir_file, "Should find files in subdirectories");
    }

    #[test]
    fn test_filter_with_relative_path() {
        let mut autocomplete = FileAutocomplete::new("/tmp/test_autocomplete");

        // Filter by path component
        autocomplete.update_filter("src".to_string());

        // Should find files containing "src" in their relative path
        assert!(!autocomplete.is_empty(), "Should find files matching 'src'");

        for file in &autocomplete.suggestions {
            assert!(
                file.relative_path.to_lowercase().contains("src"),
                "All results should contain 'src' in path: {}",
                file.relative_path
            );
        }
    }

    #[test]
    fn test_depth_limit() {
        let autocomplete = FileAutocomplete::new("/tmp/test_autocomplete");

        // All files should have depth <= max_depth (4)
        for file in &autocomplete.suggestions {
            assert!(
                file.depth <= 4,
                "File depth should not exceed 4: {} has depth {}",
                file.relative_path,
                file.depth
            );
        }
    }

    #[test]
    fn test_directory_sorting() {
        let autocomplete = FileAutocomplete::new("/tmp/test_autocomplete");

        // Directories should come before files
        let first_dir = autocomplete.suggestions.iter().find(|f| f.is_dir);
        let first_file = autocomplete.suggestions.iter().find(|f| !f.is_dir);

        if let (Some(dir), Some(file)) = (first_dir, first_file) {
            let dir_idx = autocomplete.suggestions.iter().position(|f| f.path == dir.path).unwrap();
            let file_idx = autocomplete.suggestions.iter().position(|f| f.path == file.path).unwrap();
            assert!(
                dir_idx < file_idx,
                "Directories should come before files"
            );
        }
    }
}
