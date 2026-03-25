//! TUI mode for coding-agent
//!
//! Provides a full terminal UI experience similar to Claude Code.

pub mod app;
pub mod layout;
pub mod conversation;
pub mod input;
pub mod input_status;
pub mod events;
pub mod status_bar;
pub mod autocomplete;
pub mod debug_panel;
pub mod markdown;

pub use app::TuiApp;
pub use input_status::{InputStatus, InputStatusIndicator};
