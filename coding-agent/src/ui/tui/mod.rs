//! TUI mode for coding-agent
//!
//! Provides a full terminal UI experience similar to Claude Code.

pub mod app;
pub mod layout;
pub mod conversation;
pub mod input;
pub mod events;
pub mod status_bar;

pub use app::TuiApp;
