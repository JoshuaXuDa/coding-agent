//! Cancellation token for aborting in-progress queries
//!
//! Inspired by AbortController pattern. Uses Arc<AtomicBool> for
//! cheap cloning and sharing across async tasks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A cancellation token that can be shared across tasks.
///
/// Clone cheaply and pass to async tasks. Check `is_cancelled()`
/// periodically to support graceful shutdown.
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new cancellation token in the "not cancelled" state.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Reset the token (re-use for a new query).
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancel() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_clone_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn test_reset() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
        token.reset();
        assert!(!token.is_cancelled());
    }
}
