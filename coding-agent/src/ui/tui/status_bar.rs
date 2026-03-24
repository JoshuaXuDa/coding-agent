//! Status bar widget
//!
//! Displays status information at the bottom of the TUI.

/// Status bar state
#[derive(Debug, Clone)]
pub struct StatusBar {
    /// Session ID
    pub session_id: String,
    /// Connection status
    pub connected: bool,
    /// Number of available tools
    pub tool_count: usize,
    /// Model name
    pub model: String,
    /// Currently streaming
    pub is_streaming: bool,
    /// Current status message
    pub status_message: String,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            session_id: "default".to_string(),
            connected: true,
            tool_count: 6,
            model: "glm-4-flash".to_string(),
            is_streaming: false,
            status_message: "Ready".to_string(),
        }
    }

    /// Set the status message
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
    }

    /// Set streaming state
    pub fn set_streaming(&mut self, streaming: bool) {
        self.is_streaming = streaming;
        if streaming {
            self.status_message = "Processing...".to_string();
        } else {
            self.status_message = "Ready".to_string();
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
