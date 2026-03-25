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
    /// Debug panel area (optional)
    pub debug: Option<Rect>,
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
pub fn calculate_layout(size: Rect, show_debug: bool) -> LayoutAreas {
    // Vertical layout with title, conversation, [debug], input, input status, status
    let constraints = if show_debug {
        vec![
            Constraint::Length(3),  // Title bar
            Constraint::Min(10),    // Conversation (flexible)
            Constraint::Length(15), // Debug panel
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Input status
            Constraint::Length(1),  // Status bar
        ]
    } else {
        vec![
            Constraint::Length(3),  // Title bar
            Constraint::Min(10),    // Conversation (flexible)
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Input status
            Constraint::Length(1),  // Status bar
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    // Calculate popup area - always create it, positioned above input
    let popup_height = 10u16;
    let popup_width = 50u16;
    let input_area_idx = if show_debug { 3 } else { 2 };

    let popup = Some(Rect {
        x: 2,  // Left margin
        y: chunks[input_area_idx].top().saturating_sub(popup_height + 1),
        width: popup_width.min(size.width.saturating_sub(4)),
        height: popup_height,
    });

    if show_debug {
        LayoutAreas {
            title: chunks[0],
            conversation: chunks[1],
            debug: Some(chunks[2]),
            input: chunks[3],
            input_status: chunks[4],
            status: chunks[5],
            popup,
        }
    } else {
        LayoutAreas {
            title: chunks[0],
            conversation: chunks[1],
            debug: None,
            input: chunks[2],
            input_status: chunks[3],
            status: chunks[4],
            popup,
        }
    }
}
