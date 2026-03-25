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
    /// Input status indicator area
    pub input_status: Rect,
    /// Status bar area
    pub status: Rect,
    /// Autocomplete popup area (optional)
    pub popup: Option<Rect>,
}

/// Calculate layout areas for the given terminal size
pub fn calculate_layout(size: Rect) -> LayoutAreas {
    // Vertical layout with title, conversation, input, input status, status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title bar
            Constraint::Min(10),    // Conversation (flexible)
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Input status
            Constraint::Length(1),  // Status bar
        ])
        .split(size);

    // Calculate popup area - always create it, positioned above input
    let popup_height = 10u16;
    let popup_width = 50u16;
    let popup = Some(Rect {
        x: 2,  // Left margin
        y: chunks[1].bottom().saturating_sub(popup_height + 1),
        width: popup_width,
        height: popup_height,
    });

    LayoutAreas {
        title: chunks[0],
        conversation: chunks[1],
        input: chunks[2],
        input_status: chunks[3],
        status: chunks[4],
        popup,
    }
}
