//! Query engine module
//!
//! Manages the LLM interaction lifecycle, decoupled from the TUI layer.
//! Inspired by Claude Code's QueryEngine pattern.

mod cancellation;
mod compaction;
mod engine;
mod token_estimation;
mod tirea_engine;

pub use cancellation::CancellationToken;
pub use compaction::{CompactionConfig, CompactionResult};
pub use engine::{QueryEngine, QueryEvent, QueryRequest};
pub use tirea_engine::TireaQueryEngine;
