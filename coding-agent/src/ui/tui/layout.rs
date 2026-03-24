//! Layout management for TUI
//!
//! Defines the layout constraints and areas for different widgets.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout areas for the TUI
#[derive(Debug, Clone)]
pub struct LayoutAreas {
    /// Title bar area
    pub title: Rect,
    /// Conversation area
    pub conversation: Rect,
    /// Input area
    pub input: Rect,
    /// Status bar area
    pub status: Rect,
}

/// Calculate layout areas for the given terminal size
pub fn calculate_layout(size: Rect) -> LayoutAreas {
    // Vertical layout with title, conversation, input, status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title bar
            Constraint::Min(10),    // Conversation (flexible)
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Status bar
        ])
        .split(size);

    LayoutAreas {
        title: chunks[0],
        conversation: chunks[1],
        input: chunks[2],
        status: chunks[3],
    }
}
