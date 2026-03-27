//! Text selection support for TUI panels
//!
//! Provides types and functions for mouse-based text selection
//! and clipboard copy in conversation and debug panels.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
};

/// Logical position within rendered text content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

/// Which panel a selection belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionTarget {
    Conversation,
    DebugPanel,
}

/// Active text selection state
#[derive(Debug, Clone)]
pub struct TextSelection {
    pub start: TextPosition,
    pub end: TextPosition,
    pub target: SelectionTarget,
    pub dragging: bool,
}

impl TextSelection {
    pub fn new(start: TextPosition, target: SelectionTarget) -> Self {
        Self {
            end: start,
            target,
            dragging: false,
            start,
        }
    }

    pub fn update_end(&mut self, pos: TextPosition) {
        self.end = pos;
    }

    /// Normalize selection so start <= end (line-major ordering)
    pub fn normalized(&self) -> (TextPosition, TextPosition) {
        if self.start.line < self.end.line
            || (self.start.line == self.end.line && self.start.column <= self.end.column)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Check if the selection is empty (start == end)
    pub fn is_empty(&self) -> bool {
        self.start.line == self.end.line && self.start.column == self.end.column
    }
}

/// Map absolute terminal mouse coordinates to a TextPosition within rendered content.
///
/// - `mouse_x, mouse_y`: absolute terminal coordinates from crossterm MouseEvent
/// - `area`: the Rect of the panel (from cached_areas), includes border
/// - `content_line_count`: total lines in the Text struct
/// - `scroll_offset`: lines scrolled off the top
pub fn mouse_to_text_position(
    mouse_x: u16,
    mouse_y: u16,
    area: Rect,
    content_line_count: usize,
    scroll_offset: usize,
) -> Option<TextPosition> {
    // Border consumes: left 1, right 1, top 1, bottom 1
    let content_x = mouse_x.saturating_sub(area.x + 1);
    let content_y = mouse_y.saturating_sub(area.y + 1);

    let content_width = area.width.saturating_sub(2);
    let content_height = area.height.saturating_sub(2);

    if content_x >= content_width || content_y >= content_height {
        return None;
    }

    let line_index = scroll_offset + content_y as usize;
    if line_index >= content_line_count {
        return None;
    }

    Some(TextPosition {
        line: line_index,
        column: content_x as usize,
    })
}

/// Extract plain text from the rendered Text within the selection range.
pub fn extract_selected_text(text: &Text<'_>, selection: &TextSelection) -> String {
    if selection.is_empty() {
        return String::new();
    }

    let (start, end) = selection.normalized();
    let total_lines = text.lines.len();
    let mut result = String::new();

    for line_idx in start.line..=end.line.min(total_lines.saturating_sub(1)) {
        let line = &text.lines[line_idx];
        let line_content: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        let chars: Vec<char> = line_content.chars().collect();

        let (from, to) = if line_idx == start.line && line_idx == end.line {
            let s = start.column.min(chars.len());
            let e = (end.column + 1).min(chars.len());
            (s, e)
        } else if line_idx == start.line {
            (start.column.min(chars.len()), chars.len())
        } else if line_idx == end.line {
            (0, (end.column + 1).min(chars.len()))
        } else {
            (0, chars.len())
        };

        result.extend(chars[from..to].iter());
        if line_idx != end.line.min(total_lines.saturating_sub(1)) {
            result.push('\n');
        }
    }

    result
}

/// Apply selection highlight to a Text by adding background color to selected spans.
pub fn apply_selection_highlight(text: &mut Text<'_>, selection: &TextSelection) {
    if selection.is_empty() {
        return;
    }

    let (start, end) = selection.normalized();
    let total_lines = text.lines.len();

    for line_idx in start.line..=end.line.min(total_lines.saturating_sub(1)) {
        if line_idx >= text.lines.len() {
            break;
        }

        let sel_start_col = if line_idx == start.line { start.column } else { 0 };
        let sel_end_col = if line_idx == end.line { end.column + 1 } else { usize::MAX };

        let line = &mut text.lines[line_idx];
        let mut cumulative_width = 0usize;

        for span in &mut line.spans {
            let span_width = unicode_width::UnicodeWidthStr::width(span.content.as_ref());
            let span_start = cumulative_width;
            let span_end = cumulative_width + span_width;

            // Check overlap with selection range
            if span_end > sel_start_col && span_start < sel_end_col {
                span.style = span.style.bg(Color::Rgb(80, 80, 120));
            }

            cumulative_width = span_end;
        }
    }
}
