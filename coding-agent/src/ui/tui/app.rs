//! Main TUI application
//!
//! Provides a full terminal UI experience for the coding agent.

use crate::ui::tui::{
    conversation::ChatMessage,
    events::{TuiEvent, ToolStatus},
    input::{InputMode, InputWidget},
    input_status::{InputStatus, InputStatusIndicator},
    layout::calculate_layout,
    status_bar::StatusBar,
    debug_panel::DebugPanel,
    markdown::MarkdownRenderer,
    selection::{TextSelection, SelectionTarget},
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tirea_agentos::AgentOs;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use textwrap::{wrap, Options};

/// Main TUI application
pub struct TuiApp {
    /// AgentOS instance
    agent_os: Arc<AgentOs>,
    /// Chat messages
    messages: Vec<ChatMessage>,
    /// Current reasoning content being built
    current_reasoning: String,
    /// Current assistant response being built
    current_response: String,
    /// Input widget
    input: InputWidget,
    /// Input status indicator
    input_status: InputStatusIndicator,
    /// Status bar
    status: StatusBar,
    /// Scroll offset for conversation
    scroll_offset: usize,
    /// Conversation scroll offset (for mouse scrolling)
    conversation_scroll: usize,
    /// Cached layout areas for mouse click detection
    cached_areas: Option<crate::ui::tui::layout::LayoutAreas>,
    /// Event receiver
    event_rx: mpsc::UnboundedReceiver<TuiEvent>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<TuiEvent>,
    /// Should exit
    should_exit: bool,
    /// Last tool call (for tracking completion)
    last_tool_call: Option<String>,
    /// Buffer for incomplete UTF-8 sequences
    utf8_buffer: Vec<u8>,
    /// Debug panel
    debug_panel: DebugPanel,
    /// Show debug panel
    show_debug_panel: bool,
    /// Log event receiver
    log_rx: mpsc::UnboundedReceiver<crate::logging::LogEntry>,
    /// Log event sender
    log_tx: mpsc::UnboundedSender<crate::logging::LogEntry>,
    /// Active text selection
    selection: Option<TextSelection>,
    /// Clipboard copy feedback message (text, instant)
    copy_feedback: Option<(String, std::time::Instant)>,
    /// Cached conversation line count for selection operations
    cached_conversation_line_count: usize,
    /// Cached debug panel line count for selection
    cached_debug_line_count: usize,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(agent_os: AgentOs) -> anyhow::Result<Self> {
        let agent_os = Arc::new(agent_os);
        let (event_tx, event_rx) = mpsc::unbounded_channel::<TuiEvent>();
        let (log_tx, log_rx) = mpsc::unbounded_channel::<crate::logging::LogEntry>();

        // Send startup logs to debug panel
        let _ = log_tx.send(crate::logging::LogEntry {
            level: log::Level::Info,
            module: Some("app".to_string()),
            message: "TuiApp initialized successfully".to_string(),
            timestamp: chrono::Local::now(),
        });
        let _ = log_tx.send(crate::logging::LogEntry {
            level: log::Level::Info,
            module: Some("app".to_string()),
            message: "Press F12 to open debug panel for detailed logs".to_string(),
            timestamp: chrono::Local::now(),
        });
        let _ = log_tx.send(crate::logging::LogEntry {
            level: log::Level::Info,
            module: Some("app".to_string()),
            message: "Debug panel shows all agent execution logs".to_string(),
            timestamp: chrono::Local::now(),
        });

        Ok(Self {
            agent_os,
            messages: Vec::new(),
            current_reasoning: String::new(),
            current_response: String::new(),
            input: InputWidget::new(),
            input_status: InputStatusIndicator::new(),
            status: StatusBar::new(),
            scroll_offset: 0,
            conversation_scroll: 0,
            cached_areas: None,
            event_rx,
            event_tx,
            should_exit: false,
            last_tool_call: None,
            utf8_buffer: Vec::new(),
            debug_panel: DebugPanel::new(1000),
            show_debug_panel: false,
            log_rx,
            log_tx: log_tx.clone(),
            selection: None,
            copy_feedback: None,
            cached_conversation_line_count: 0,
            cached_debug_line_count: 0,
        })
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the main loop
        let result = self.run_main_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Main application loop
    async fn run_main_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<()> {
        let mut tick_interval = interval(Duration::from_millis(250));

        loop {
            // Draw the UI
            terminal.draw(|f| self.draw(f))?;

            // Handle events
            use tokio::select;

            tokio::select! {
                // Keyboard and mouse input
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if event::poll(Duration::from_millis(50))? {
                        match event::read()? {
                            event::Event::Key(key) => self.handle_key_event(key),
                            event::Event::Mouse(mouse) => self.handle_mouse_event(mouse),
                            _ => {}
                        }
                    }
                }

                // TUI events (from agent)
                Some(event) = self.event_rx.recv() => {
                    self.handle_tui_event(event);
                }

                // Log events for debug panel
                Some(log_entry) = self.log_rx.recv() => {
                    if self.show_debug_panel {
                        self.debug_panel.add_log(log_entry);
                    }
                }

                // Tick for periodic updates
                _ = tick_interval.tick() => {
                    // Could update time, etc.
                }
            }

            if self.should_exit {
                break;
            }
        }

        Ok(())
    }

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                // If in autocomplete mode, cancel it
                if self.input.mode() == &InputMode::Autocomplete {
                    self.input.set_mode(InputMode::Normal);
                    self.input.autocomplete = None;
                    self.input.autocomplete_trigger_pos = None;
                } else {
                    // Otherwise exit
                    self.should_exit = true;
                }
            }
            KeyCode::Char('q') => {
                self.should_exit = true;
            }
            KeyCode::F(12) => {
                // Toggle debug panel
                self.show_debug_panel = !self.show_debug_panel;
            }
            KeyCode::Char('l') => {
                // Cycle log level filter if debug panel is visible
                if self.show_debug_panel {
                    self.debug_panel.cycle_level_filter();
                }
            }
            KeyCode::Char('c') => {
                // Clear debug panel if visible
                if self.show_debug_panel {
                    self.debug_panel.clear();
                }
            }
            KeyCode::PageUp => {
                if self.input.is_autocomplete_active() {
                    // Let input widget handle autocomplete navigation
                    if self.input.handle_key_event(key) {
                        self.send_message();
                    }
                } else if self.show_debug_panel {
                    self.debug_panel.page_up();
                }
            }
            KeyCode::PageDown => {
                if self.input.is_autocomplete_active() {
                    // Let input widget handle autocomplete navigation
                    if self.input.handle_key_event(key) {
                        self.send_message();
                    }
                } else if self.show_debug_panel {
                    self.debug_panel.page_down();
                }
            }
            KeyCode::Up => {
                if self.input.is_autocomplete_active() {
                    // Let input widget handle autocomplete navigation
                    if self.input.handle_key_event(key) {
                        self.send_message();
                    }
                } else if self.show_debug_panel {
                    self.debug_panel.scroll_up();
                }
            }
            KeyCode::Down => {
                if self.input.is_autocomplete_active() {
                    // Let input widget handle autocomplete navigation
                    if self.input.handle_key_event(key) {
                        self.send_message();
                    }
                } else if self.show_debug_panel {
                    self.debug_panel.scroll_down();
                }
            }
            _ => {
                // Check for Ctrl+T to toggle thinking block
                if key.code == KeyCode::Char('t')
                    && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    self.toggle_last_thinking();
                    return;
                }
                // Clear selection on key input
                self.selection = None;
                // Pass to input widget for all other keys
                if self.input.handle_key_event(key) {
                    // User wants to send the message (only happens on Enter)
                    self.send_message();
                }
            }
        }
    }

    /// Handle mouse events
    fn handle_mouse_event(&mut self, event: crossterm::event::MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};
        use crate::ui::tui::selection::TextPosition;

        match event.kind {
            // Scroll wheel - scroll conversation or debug panel
            MouseEventKind::ScrollUp => {
                self.selection = None;
                if self.is_in_debug_panel(event.column, event.row) {
                    self.debug_panel.scroll_up();
                } else if self.is_in_conversation_area(event.column, event.row) {
                    self.conversation_scroll = self.conversation_scroll.saturating_sub(1);
                }
            }
            MouseEventKind::ScrollDown => {
                self.selection = None;
                if self.is_in_debug_panel(event.column, event.row) {
                    self.debug_panel.scroll_down();
                } else if self.is_in_conversation_area(event.column, event.row) {
                    self.conversation_scroll = self.conversation_scroll.saturating_add(1);
                }
            }

            // Left click - toggle thinking blocks or start selection
            MouseEventKind::Down(MouseButton::Left) => {
                if self.is_click_on_thinking_block(event.column, event.row) {
                    self.toggle_last_thinking();
                    self.selection = None;
                    return;
                }

                // Start text selection
                let target = if self.is_in_debug_panel(event.column, event.row) {
                    if let Some(debug_area) = self.cached_areas.as_ref().and_then(|a| a.debug) {
                        let pos = crate::ui::tui::selection::mouse_to_text_position(
                            event.column, event.row,
                            debug_area, self.cached_debug_line_count,
                            self.debug_panel.scroll_offset(),
                        );
                        pos.map(|p| (p, SelectionTarget::DebugPanel))
                    } else {
                        None
                    }
                } else if self.is_in_conversation_area(event.column, event.row) {
                    let line_count = self.cached_conversation_line_count;
                    let pos = crate::ui::tui::selection::mouse_to_text_position(
                        event.column, event.row,
                        self.cached_areas.as_ref().unwrap().conversation,
                        line_count,
                        0, // scroll already applied in draw_conversation via .skip()
                    );
                    pos.map(|p| (p, SelectionTarget::Conversation))
                } else {
                    None
                };

                if let Some((pos, tgt)) = target {
                    self.selection = Some(TextSelection::new(pos, tgt));
                } else {
                    self.selection = None;
                }
            }

            // Mouse drag - update selection end
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(sel) = &mut self.selection {
                    sel.dragging = true;
                    let (area, line_count, scroll) = match sel.target {
                        SelectionTarget::Conversation => (
                            self.cached_areas.as_ref().map(|a| a.conversation),
                            self.cached_conversation_line_count,
                            0, // scroll already applied in draw_conversation via .skip()
                        ),
                        SelectionTarget::DebugPanel => (
                            self.cached_areas.as_ref().and_then(|a| a.debug),
                            self.cached_debug_line_count,
                            self.debug_panel.scroll_offset(),
                        ),
                    };
                    if let (Some(area), lc, sc) = (area, line_count, scroll) {
                        if let Some(pos) = crate::ui::tui::selection::mouse_to_text_position(
                            event.column, event.row, area, lc, sc,
                        ) {
                            sel.update_end(pos);
                        }
                    }
                }
            }

            // Mouse release - copy selected text to clipboard
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(ref sel) = self.selection {
                    if sel.dragging && !sel.is_empty() {
                        let content: Option<String> = match sel.target {
                            SelectionTarget::Conversation => {
                                self.build_conversation_text_for_selection()
                                    .and_then(|t| {
                                        let extracted = crate::ui::tui::selection::extract_selected_text(&t, sel);
                                        if extracted.is_empty() { None } else { Some(extracted) }
                                    })
                            }
                            SelectionTarget::DebugPanel => {
                                // Debug panel text extraction not cached; skip for now
                                None
                            }
                        };

                        if let Some(content) = content {
                            if !content.is_empty() {
                                let char_count = content.chars().count();
                                match arboard::Clipboard::new() {
                                    Ok(mut cb) => {
                                        match cb.set_text(&content) {
                                            Ok(()) => self.copy_feedback = Some((
                                                format!("已复制 {} 字", char_count),
                                                std::time::Instant::now(),
                                            )),
                                            Err(_) => self.copy_feedback = Some((
                                                "复制失败".to_string(),
                                                std::time::Instant::now(),
                                            )),
                                        }
                                    }
                                    Err(_) => self.copy_feedback = Some((
                                        "剪贴板不可用".to_string(),
                                        std::time::Instant::now(),
                                    )),
                                }
                            }
                        }
                    }
                }
                // Stop dragging but keep selection visible
                if let Some(ref mut sel) = self.selection {
                    sel.dragging = false;
                }
            }

            _ => {}
        }
    }

    /// Check if mouse position is in conversation area
    fn is_in_conversation_area(&self, x: u16, y: u16) -> bool {
        if let Some(areas) = &self.cached_areas {
            let conv = areas.conversation;
            x >= conv.x && x < conv.x + conv.width
                && y >= conv.y && y < conv.y + conv.height
        } else {
            false
        }
    }

    /// Check if mouse position is in debug panel
    fn is_in_debug_panel(&self, x: u16, y: u16) -> bool {
        if !self.show_debug_panel {
            return false;
        }
        if let Some(areas) = &self.cached_areas {
            if let Some(debug) = areas.debug {
                x >= debug.x && x < debug.x + debug.width
                    && y >= debug.y && y < debug.y + debug.height
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Check if click is on a thinking block
    fn is_click_on_thinking_block(&self, x: u16, y: u16) -> bool {
        // Simple check: if we have thinking messages and click is in conversation area
        // This is a basic implementation - could be enhanced with precise region tracking
        if !self.is_in_conversation_area(x, y) {
            return false;
        }

        // Check if we have any thinking messages
        self.messages.iter().any(|msg| msg.is_thinking())
    }

    /// Handle TUI events
    fn handle_tui_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::AgentReasoning(delta) => {
                self.selection = None;
                // Filter out standalone empty lines
                if delta == "\n" && self.current_reasoning.is_empty() {
                    return;
                }

                // Append reasoning content
                self.append_reasoning_delta(&delta);
                self.status.set_streaming(true);
            }
            TuiEvent::AgentText(delta) => {
                self.selection = None;
                // Filter out standalone empty lines
                if delta == "\n" && self.current_response.is_empty() {
                    return;
                }

                // Append response content
                self.append_text_delta(&delta);
                self.status.set_streaming(true);
            }
            TuiEvent::AgentToolCall { name, .. } => {
                self.messages.push(ChatMessage::ToolCall {
                    name: name.clone(),
                    status: ToolStatus::Running,
                });
                self.last_tool_call = Some(name.clone());
                self.status.set_status(format!("Running tool: {}", name));
            }
            TuiEvent::AgentToolDone { name } => {
                // Update the tool call status
                for msg in self.messages.iter_mut().rev() {
                    if let ChatMessage::ToolCall { name: n, status } = msg {
                        if n == &name {
                            *status = ToolStatus::Done;
                            break;
                        }
                    }
                }
                self.status.set_status("Ready");
            }
            TuiEvent::AgentError(err) => {
                // Save reasoning if exists
                if !self.current_reasoning.is_empty() {
                    self.messages.push(ChatMessage::Thinking {
                        content: self.current_reasoning.clone(),
                        expanded: false,
                    });
                    self.current_reasoning = String::new();
                }

                // Save response if exists
                if !self.current_response.is_empty() {
                    self.messages.push(ChatMessage::Assistant {
                        content: self.current_response.clone(),
                    });
                    self.current_response = String::new();
                }

                // Add error message
                self.messages.push(ChatMessage::System {
                    content: format!("❌ 错误: {}", err),
                });

                self.status.set_streaming(false);
                self.status.set_status("Error");
            }
            TuiEvent::AgentResponseComplete => {
                // Save reasoning if exists
                if !self.current_reasoning.is_empty() {
                    self.messages.push(ChatMessage::Thinking {
                        content: self.current_reasoning.clone(),
                        expanded: false,
                    });
                    self.current_reasoning = String::new();
                }

                // Save response if exists
                if !self.current_response.is_empty() {
                    self.messages.push(ChatMessage::Assistant {
                        content: self.current_response.clone(),
                    });
                    self.current_response = String::new();
                }

                // Update status
                self.status.set_streaming(false);
                self.input_status.set_status(InputStatus::Sent);
            }
            TuiEvent::Tick => {
                // Periodic updates
            }
            TuiEvent::Input(_) => {
                // Handled in key event handler
            }
            TuiEvent::Mouse(_) => {
                // Mouse events are handled in the main loop, not via TuiEvent channel
                // This variant is here for completeness but shouldn't be used
            }
        }
    }

    /// Send the current message to the agent
    fn send_message(&mut self) {
        let text = self.input.text().trim().to_string();

        if text.is_empty() {
            return;
        }

        // Set sending status
        self.input_status.set_status(InputStatus::Sending);

        // Add user message
        self.messages.push(ChatMessage::User { content: text.clone() });
        self.input.clear();

        // Start new reasoning and response buffers
        self.current_reasoning = String::new();
        self.current_response = String::new();

        // Spawn agent task to process the message
        let agent_os = self.agent_os.clone();
        let event_tx = self.event_tx.clone();
        let log_tx = self.log_tx.clone();
        let message = text.clone();

        let _ = log_tx.send(crate::logging::LogEntry {
            level: log::Level::Info,
            module: Some("app".to_string()),
            message: format!("Starting agent task for message: {}", message),
            timestamp: chrono::Local::now(),
        });

        tokio::spawn(async move {
            use tirea::prelude::Message;
            use tirea_contract::RunRequest;
            use futures::StreamExt;
            use tirea::contracts::AgentEvent;

            let run_request = RunRequest {
                agent_id: "coding-agent".to_string(),
                thread_id: None,
                run_id: None,
                parent_run_id: None,
                parent_thread_id: None,
                resource_id: None,
                origin: Default::default(),
                state: None,
                messages: vec![Message::user(message)],
                initial_decisions: vec![],
            };

            let _ = log_tx.send(crate::logging::LogEntry {
                level: log::Level::Info,
                module: Some("app".to_string()),
                message: format!("Calling agent_os.run_stream() with agent_id: {}", run_request.agent_id),
                timestamp: chrono::Local::now(),
            });
            let _ = log_tx.send(crate::logging::LogEntry {
                level: log::Level::Debug,
                module: Some("app".to_string()),
                message: format!("Thread ID: {:?}", run_request.thread_id),
                timestamp: chrono::Local::now(),
            });

            match agent_os.run_stream(run_request).await {
                Ok(mut stream) => {
                    let _ = log_tx.send(crate::logging::LogEntry {
                        level: log::Level::Info,
                        module: Some("app".to_string()),
                        message: "Agent stream created successfully, waiting for events...".to_string(),
                        timestamp: chrono::Local::now(),
                    });

                    let mut event_count = 0u32;

                    while let Some(event) = stream.events.next().await {
                        event_count += 1;
                        let _ = log_tx.send(crate::logging::LogEntry {
                            level: log::Level::Debug,
                            module: Some("app".to_string()),
                            message: format!("Event #{}: {:?}", event_count, std::mem::discriminant(&event)),
                            timestamp: chrono::Local::now(),
                        });

                        match event {
                            AgentEvent::TextDelta { delta, .. } => {
                                let _ = log_tx.send(crate::logging::LogEntry {
                                    level: log::Level::Debug,
                                    module: Some("app".to_string()),
                                    message: format!("TextDelta: {} chars", delta.len()),
                                    timestamp: chrono::Local::now(),
                                });
                                let _ = event_tx.send(TuiEvent::AgentText(delta));
                            }
                            AgentEvent::ReasoningDelta { delta, .. } => {
                                let _ = log_tx.send(crate::logging::LogEntry {
                                    level: log::Level::Debug,
                                    module: Some("app".to_string()),
                                    message: format!("ReasoningDelta: {} chars", delta.len()),
                                    timestamp: chrono::Local::now(),
                                });
                                let _ = event_tx.send(TuiEvent::AgentReasoning(delta));
                            }
                            AgentEvent::ToolCallStart { name, .. } => {
                                let _ = log_tx.send(crate::logging::LogEntry {
                                    level: log::Level::Info,
                                    module: Some("app".to_string()),
                                    message: format!("Tool call started: {}", name),
                                    timestamp: chrono::Local::now(),
                                });
                                let _ = event_tx.send(TuiEvent::AgentToolCall {
                                    name: name.clone(),
                                    input: serde_json::json!({}),
                                });
                            }
                            AgentEvent::ToolCallDone { .. } => {
                                let _ = log_tx.send(crate::logging::LogEntry {
                                    level: log::Level::Info,
                                    module: Some("app".to_string()),
                                    message: "Tool call completed".to_string(),
                                    timestamp: chrono::Local::now(),
                                });
                                let _ = event_tx.send(TuiEvent::AgentToolDone {
                                    name: "tool".to_string(),
                                });
                            }
                            AgentEvent::Error { message, .. } => {
                                let _ = log_tx.send(crate::logging::LogEntry {
                                    level: log::Level::Error,
                                    module: Some("app".to_string()),
                                    message: format!("Agent error event: {}", message),
                                    timestamp: chrono::Local::now(),
                                });
                                let _ = event_tx.send(TuiEvent::AgentError(message));
                            }
                            _ => {
                                let _ = log_tx.send(crate::logging::LogEntry {
                                    level: log::Level::Debug,
                                    module: Some("app".to_string()),
                                    message: format!("Other event type: {:?}", std::mem::discriminant(&event)),
                                    timestamp: chrono::Local::now(),
                                });
                            }
                        }
                    }

                    let _ = log_tx.send(crate::logging::LogEntry {
                        level: log::Level::Info,
                        module: Some("app".to_string()),
                        message: format!("Agent stream ended. Total events received: {}", event_count),
                        timestamp: chrono::Local::now(),
                    });

                    // Send response complete event
                    let _ = event_tx.send(TuiEvent::AgentResponseComplete);
                }
                Err(e) => {
                    let _ = log_tx.send(crate::logging::LogEntry {
                        level: log::Level::Error,
                        module: Some("app".to_string()),
                        message: format!("Failed to create agent stream: {}", e),
                        timestamp: chrono::Local::now(),
                    });
                    let _ = log_tx.send(crate::logging::LogEntry {
                        level: log::Level::Error,
                        module: Some("app".to_string()),
                        message: format!("Error type: {:?}", std::error::Error::source(&e)),
                        timestamp: chrono::Local::now(),
                    });
                    let _ = event_tx.send(TuiEvent::AgentError(e.to_string()));
                }
            }

            let _ = log_tx.send(crate::logging::LogEntry {
                level: log::Level::Info,
                module: Some("app".to_string()),
                message: "Agent task completed".to_string(),
                timestamp: chrono::Local::now(),
            });
        });

        self.status.set_streaming(true);
    }

    /// Append text delta with UTF-8 boundary handling
    fn append_text_delta(&mut self, delta: &str) {
        // Extend the buffer with new bytes
        self.utf8_buffer.extend_from_slice(delta.as_bytes());

        // Try to convert the entire buffer to a string
        match String::from_utf8(std::mem::take(&mut self.utf8_buffer)) {
            Ok(text) => {
                // All bytes are valid UTF-8
                self.current_response.push_str(&text);
                // Buffer is already empty due to mem::take
            }
            Err(err) => {
                // Some bytes are invalid UTF-8
                let valid_len = err.utf8_error().valid_up_to();
                let bytes = err.into_bytes();
                if valid_len > 0 {
                    // Append the valid portion
                    let valid_text = String::from_utf8_lossy(&bytes[..valid_len]);
                    self.current_response.push_str(&valid_text);
                }
                // Keep the incomplete bytes in buffer
                self.utf8_buffer = bytes[valid_len..].to_vec();
            }
        }
    }

    /// Append reasoning delta with UTF-8 boundary handling
    fn append_reasoning_delta(&mut self, delta: &str) {
        // Extend the buffer with new bytes
        self.utf8_buffer.extend_from_slice(delta.as_bytes());

        // Try to convert the entire buffer to a string
        match String::from_utf8(std::mem::take(&mut self.utf8_buffer)) {
            Ok(text) => {
                // All bytes are valid UTF-8
                self.current_reasoning.push_str(&text);
                // Buffer is already empty due to mem::take
            }
            Err(err) => {
                // Some bytes are invalid UTF-8
                let valid_len = err.utf8_error().valid_up_to();
                let bytes = err.into_bytes();
                if valid_len > 0 {
                    // Append the valid portion
                    let valid_text = String::from_utf8_lossy(&bytes[..valid_len]);
                    self.current_reasoning.push_str(&valid_text);
                }
                // Keep the incomplete bytes in buffer
                self.utf8_buffer = bytes[valid_len..].to_vec();
            }
        }
    }

    /// Toggle the last thinking block's expanded state
    fn toggle_last_thinking(&mut self) {
        // Find the last thinking message and toggle its state
        for msg in self.messages.iter_mut().rev() {
            if let ChatMessage::Thinking { expanded, .. } = msg {
                *expanded = !*expanded;
                break;
            }
        }
    }

    /// Wrap text to fit within a given width, handling UTF-8 correctly
    fn wrap_text(text: &str, width: usize) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }

        // Calculate available width (subtract prefix like "Agent: " or "You: ")
        let available_width = width.saturating_sub(8);

        if available_width < 10 {
            // Too narrow, just return as-is
            return vec![text.to_string()];
        }

        // Use textwrap with Unicode-aware word boundaries
        let options = Options::new(available_width)
            .break_words(false);

        wrap(text, options)
            .into_iter()
            .map(|line| line.into_owned())
            .collect()
    }

    /// Draw the UI
    fn draw(&mut self, frame: &mut Frame) {
        let size = frame.size();
        let areas = calculate_layout(size, self.show_debug_panel);

        // Cache layout areas for mouse event handling
        self.cached_areas = Some(areas.clone());

        // Draw title bar
        self.draw_title_bar(frame, areas.title);

        // Draw conversation
        self.draw_conversation(frame, areas.conversation);

        // Draw debug panel if enabled
        if self.show_debug_panel {
            if let Some(debug_area) = areas.debug {
                // Count visible debug lines before rendering for selection
                let visible_height = debug_area.height.saturating_sub(2) as usize;
                let start_idx = self.debug_panel.scroll_offset();
                let total_filtered = self.debug_panel.filtered_count();
                self.cached_debug_line_count = total_filtered;
                self.debug_panel.render(frame, debug_area);
            }
        }

        // Draw input
        self.input.render(frame, areas.input);

        // Draw input status indicator
        self.input_status.render(frame, areas.input_status);

        // Draw status bar
        self.draw_status_bar(frame, areas.status);

        // Draw autocomplete popup if active
        if self.input.is_autocomplete_active() {
            if let Some(autocomplete) = &self.input.autocomplete {
                // Use calculated popup area, or create a default one
                let popup_area = areas.popup.unwrap_or_else(|| {
                    // Fallback: create popup area above input box
                    let popup_height = 10.min(areas.conversation.height.saturating_sub(2));
                    Rect {
                        x: areas.input.x,
                        y: areas.input.top().saturating_sub(popup_height),
                        width: 50.min(areas.input.width),
                        height: popup_height,
                    }
                });
                autocomplete.render(frame, popup_area);
            }
        }
    }

    /// Draw the title bar
    fn draw_title_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let title = vec![
            Line::from(vec![
                Span::styled("🤖 CodingAgent v1.0", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled(format!("Session: {}", self.status.session_id), Style::default().fg(Color::White)),
            ]),
        ];

        let widget = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        frame.render_widget(widget, area);
    }

    /// Draw the conversation area
    fn draw_conversation(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        use ratatui::text::{Text,};

        let mut text = Text::default();
        let available_width = area.width.saturating_sub(4) as usize;
        let mut last_message_type: Option<&str> = None;

        // Apply scroll offset - skip messages that are above the visible area
        let visible_messages: Vec<_> = self.messages.iter()
            .skip(self.conversation_scroll)
            .collect();

        for msg in visible_messages {
            // Add separator between different message types
            if let Some(last_type) = last_message_type {
                let current_type = match msg {
                    ChatMessage::User { .. } => "user",
                    ChatMessage::Assistant { .. } => "assistant",
                    _ => continue,
                };

                if last_type != current_type {
                    // Add visual separator
                    let separator = "─".repeat(available_width.min(40));
                    text.extend(vec![
                        Line::from(vec![
                            Span::styled(separator.clone(), Style::default().fg(Color::DarkGray)),
                        ]),
                        Line::from(""),
                    ]);
                }
            }

            match msg {
                ChatMessage::User { content } => {
                    let wrapped_lines = Self::wrap_text(content, available_width);
                    for (i, line) in wrapped_lines.iter().enumerate() {
                        if i == 0 {
                            text.push_line(Line::from(vec![
                                Span::styled("You: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                                Span::styled(line.clone(), Style::default().fg(Color::White)),
                            ]));
                        } else {
                            text.push_line(Line::from(vec![
                                Span::styled("      ", Style::default()),
                                Span::styled(line.clone(), Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                    last_message_type = Some("user");
                }
                ChatMessage::Thinking { content, expanded } => {
                    // Show thinking block with expand/collapse indicator
                    if *expanded {
                        // Expanded: show content with markdown rendering
                        let rendered = MarkdownRenderer::render(content, available_width);
                        text.push_line(Line::from(vec![
                            Span::styled("💭 [思考内容 - 按Ctrl+T折叠]", Style::default().fg(Color::DarkGray)),
                        ]));
                        // Add indentation prefix to each line
                        for mut line in rendered.lines {
                            line.spans.insert(0, Span::styled("  ", Style::default()));
                            text.push_line(line);
                        }
                    } else {
                        // Collapsed: show brief indicator
                        text.push_line(Line::from(vec![
                            Span::styled("💭 ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("[思考内容已折叠，按Ctrl+T展开 - {}字]", content.chars().count()),
                                Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
                            ),
                        ]));
                    }
                    text.push_line(Line::from(""));
                    last_message_type = Some("thinking");
                }
                ChatMessage::Assistant { content } => {
                    // Use markdown rendering for assistant messages
                    let rendered = MarkdownRenderer::render(content, available_width);
                    text.push_line(Line::from(vec![
                        Span::styled("Agent: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    ]));
                    // Add indentation prefix to each line
                    for mut line in rendered.lines {
                        line.spans.insert(0, Span::styled("       ", Style::default()));
                        text.push_line(line);
                    }
                    last_message_type = Some("assistant");
                }
                ChatMessage::ToolCall { name, status } => {
                    let status_icon = match status {
                        ToolStatus::Running => "⏳",
                        ToolStatus::Done => "✓",
                        ToolStatus::Error(_) => "✗",
                    };

                    text.push_line(Line::from(vec![
                        Span::styled("┌─ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(status_icon, Style::default().fg(Color::Yellow)),
                        Span::styled(" ", Style::default()),
                        Span::styled(name.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    ]));
                }
                ChatMessage::System { content } => {
                    text.push_line(Line::from(vec![
                        Span::styled("System: ", Style::default().fg(Color::Gray)),
                        Span::styled(content.clone(), Style::default().fg(Color::Gray)),
                    ]));
                }
            }
        }

        // Add current streaming reasoning with wrapping
        if !self.current_reasoning.is_empty() {
            text.push_line(Line::from(vec![
                Span::styled("💭 [思考中...]", Style::default().fg(Color::DarkGray)),
            ]));
            let wrapped_lines = Self::wrap_text(&self.current_reasoning, available_width);
            let line_count = wrapped_lines.len();
            for (i, line) in wrapped_lines.iter().enumerate() {
                let is_last = i == line_count - 1;
                text.push_line(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(line.clone(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)),
                    if is_last {
                        Span::styled("▌", Style::default().fg(Color::DarkGray))
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));
            }
            text.push_line(Line::from(""));
        }

        // Add current streaming response with wrapping
        if !self.current_response.is_empty() {
            let wrapped_lines = Self::wrap_text(&self.current_response, available_width);
            let line_count = wrapped_lines.len();
            for (i, line) in wrapped_lines.iter().enumerate() {
                let is_last = i == line_count - 1;
                if i == 0 {
                    text.push_line(Line::from(vec![
                        Span::styled("Agent: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled(line.clone(), Style::default().fg(Color::White)),
                        if is_last {
                            Span::styled("▌", Style::default().fg(Color::DarkGray))
                        } else {
                            Span::styled("", Style::default())
                        },
                    ]));
                } else {
                    text.push_line(Line::from(vec![
                        Span::styled("       ", Style::default()),
                        Span::styled(line.clone(), Style::default().fg(Color::White)),
                        if is_last {
                            Span::styled("▌", Style::default().fg(Color::DarkGray))
                        } else {
                            Span::styled("", Style::default())
                        },
                    ]));
                }
            }
        }

        // Cache line count and apply selection highlight
        self.cached_conversation_line_count = text.lines.len();
        if let Some(ref sel) = self.selection {
            if sel.target == SelectionTarget::Conversation {
                crate::ui::tui::selection::apply_selection_highlight(&mut text, sel);
            }
        }

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Conversation"));

        frame.render_widget(paragraph, area);
    }

    /// Build an owned Text from messages for text extraction during copy.
    fn build_conversation_text_for_selection(&self) -> Option<ratatui::text::Text<'static>> {
        use ratatui::text::{Span, Text};

        let available_width = self.cached_areas.as_ref()
            .map(|a| a.conversation.width.saturating_sub(4) as usize)?;

        let mut text = Text::default();
        let visible_messages: Vec<_> = self.messages.iter()
            .skip(self.conversation_scroll)
            .collect();

        for msg in visible_messages {
            match msg {
                ChatMessage::User { content } => {
                    let wrapped = Self::wrap_text(content, available_width);
                    for (i, line) in wrapped.iter().enumerate() {
                        let prefix = if i == 0 { "You: " } else { "      " };
                        text.push_line(Line::from(Span::raw(format!("{}{}", prefix, line))));
                    }
                }
                ChatMessage::Assistant { content } => {
                    text.push_line(Line::from(Span::raw("Agent: ")));
                    let rendered = MarkdownRenderer::render(content, available_width);
                    for line in rendered.lines {
                        let plain: String = line.spans.iter().map(|s| s.content.clone()).collect();
                        text.push_line(Line::from(Span::raw(format!("       {}", plain))));
                    }
                }
                ChatMessage::Thinking { content, expanded } => {
                    if *expanded {
                        text.push_line(Line::from(Span::raw("💭 [思考内容]")));
                        let rendered = MarkdownRenderer::render(content, available_width);
                        for line in rendered.lines {
                            let plain: String = line.spans.iter().map(|s| s.content.clone()).collect();
                            text.push_line(Line::from(Span::raw(format!("  {}", plain))));
                        }
                    } else {
                        text.push_line(Line::from(Span::raw(format!(
                            "💭 [思考内容已折叠 - {}字]",
                            content.chars().count()
                        ))));
                    }
                    text.push_line(Line::from(""));
                }
                ChatMessage::ToolCall { name, status } => {
                    let icon = match status {
                        ToolStatus::Running => "⏳",
                        ToolStatus::Done => "✓",
                        ToolStatus::Error(_) => "✗",
                    };
                    text.push_line(Line::from(Span::raw(format!("┌─ {} {}", icon, name))));
                }
                ChatMessage::System { content } => {
                    text.push_line(Line::from(Span::raw(format!("System: {}", content))));
                }
            }
        }

        // Add current streaming content
        if !self.current_reasoning.is_empty() {
            text.push_line(Line::from(Span::raw("💭 [思考中...]")));
            let wrapped = Self::wrap_text(&self.current_reasoning, available_width);
            for line in &wrapped {
                text.push_line(Line::from(Span::raw(format!("  {}", line))));
            }
            text.push_line(Line::from(""));
        }
        if !self.current_response.is_empty() {
            let wrapped = Self::wrap_text(&self.current_response, available_width);
            for (i, line) in wrapped.iter().enumerate() {
                let prefix = if i == 0 { "Agent: " } else { "       " };
                text.push_line(Line::from(Span::raw(format!("{}{}", prefix, line))));
            }
        }

        Some(text)
    }

    /// Draw the status bar
    fn draw_status_bar(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        // Check if we have copy feedback to show (expires after 3 seconds)
        let feedback = self.copy_feedback.as_ref()
            .filter(|(_, instant)| instant.elapsed() < std::time::Duration::from_secs(3));

        if let Some((msg, _)) = feedback {
            let widget = Paragraph::new(Line::from(vec![
                Span::styled("📋 ", Style::default()),
                Span::styled(msg.clone(), Style::default().fg(Color::Green)),
            ]));
            frame.render_widget(widget, area);
            return;
        }

        // Expire old feedback
        self.copy_feedback = None;
        let help_text = "Enter:发送 滚轮/↑↓:滚动 点击/Ctrl+T:切换 ESC:退出";

        let status_text = if self.status.is_streaming {
            format!("⏳ {} | {} | Tools: {} | Model: {}",
                help_text,
                "处理中",
                self.status.tool_count,
                self.status.model
            )
        } else {
            format!("💡 {} | {} | Tools: {} | Model: {}",
                help_text,
                self.status.status_message,
                self.status.tool_count,
                self.status.model
            )
        };

        let widget = Paragraph::new(Line::from(vec![
            Span::styled(status_text, Style::default().fg(Color::DarkGray)),
        ]))
        .wrap(Wrap { trim: true });

        frame.render_widget(widget, area);
    }

    /// Get the event channel sender for external use
    pub fn event_tx(&self) -> mpsc::UnboundedSender<TuiEvent> {
        self.event_tx.clone()
    }

    /// Get the log channel sender for external use
    pub fn log_tx(&self) -> mpsc::UnboundedSender<crate::logging::LogEntry> {
        self.log_tx.clone()
    }
}
