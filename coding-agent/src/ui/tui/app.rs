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

/// Main TUI application
pub struct TuiApp {
    /// AgentOS instance
    agent_os: Arc<AgentOs>,
    /// Chat messages
    messages: Vec<ChatMessage>,
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
    /// Event receiver
    event_rx: mpsc::UnboundedReceiver<TuiEvent>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<TuiEvent>,
    /// Should exit
    should_exit: bool,
    /// Last tool call (for tracking completion)
    last_tool_call: Option<String>,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new(agent_os: AgentOs) -> anyhow::Result<Self> {
        let agent_os = Arc::new(agent_os);
        let (event_tx, event_rx) = mpsc::unbounded_channel::<TuiEvent>();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            agent_os,
            messages: Vec::new(),
            current_response: String::new(),
            input: InputWidget::new(),
            input_status: InputStatusIndicator::new(),
            status: StatusBar::new(),
            scroll_offset: 0,
            event_rx,
            event_tx,
            should_exit: false,
            last_tool_call: None,
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
                // Keyboard input
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if event::poll(Duration::from_millis(50))? {
                        if let event::Event::Key(key) = event::read()? {
                            self.handle_key_event(key);
                        }
                    }
                }

                // TUI events (from agent)
                Some(event) = self.event_rx.recv() => {
                    self.handle_tui_event(event);
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
                // If in file selector mode, cancel it
                if self.input.mode() == &InputMode::FileSelector {
                    self.input.set_mode(InputMode::Normal);
                    // Remove the @ from input
                    let text = self.input.text();
                    if text.ends_with('@') {
                        let new_text = text[..text.len()-1].to_string();
                        self.input.clear();
                        // Re-insert without @
                        for c in new_text.chars() {
                            self.input.insert_char(c);
                        }
                    }
                } else {
                    // Otherwise exit
                    self.should_exit = true;
                }
            }
            KeyCode::Char('q') => {
                self.should_exit = true;
            }
            KeyCode::Char('c') => {
                // Could be Ctrl+C, handled by crossterm
            }
            _ => {
                // Pass to input widget for all other keys
                if self.input.handle_key_event(key) {
                    // User wants to send the message (only happens on Enter)
                    self.send_message();
                }
            }
        }
    }

    /// Handle TUI events
    fn handle_tui_event(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::AgentText(delta) => {
                // 过滤掉单独的空行（如果响应为空）
                if delta == "\n" && self.current_response.is_empty() {
                    return;
                }

                self.current_response.push_str(&delta);
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
                // 如果有流式响应，先保存
                if !self.current_response.is_empty() {
                    self.messages.push(ChatMessage::Assistant {
                        content: self.current_response.clone(),
                    });
                    self.current_response = String::new();
                }

                // 添加错误消息
                self.messages.push(ChatMessage::System {
                    content: format!("❌ 错误: {}", err),
                });

                self.status.set_streaming(false);
                self.status.set_status("Error");
            }
            TuiEvent::AgentResponseComplete => {
                // 将流式响应保存为正式消息
                if !self.current_response.is_empty() {
                    self.messages.push(ChatMessage::Assistant {
                        content: self.current_response.clone(),
                    });
                    self.current_response = String::new();
                }

                // 更新状态
                self.status.set_streaming(false);
                self.input_status.set_status(InputStatus::Sent);
            }
            TuiEvent::Tick => {
                // Periodic updates
            }
            TuiEvent::Input(_) => {
                // Handled in key event handler
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

        // Start a new assistant response
        self.current_response = String::new();

        // Spawn agent task to process the message
        let agent_os = self.agent_os.clone();
        let event_tx = self.event_tx.clone();
        let message = text;

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

            match agent_os.run_stream(run_request).await {
                Ok(mut stream) => {
                    while let Some(event) = stream.events.next().await {
                        match event {
                            AgentEvent::TextDelta { delta, .. } => {
                                let _ = event_tx.send(TuiEvent::AgentText(delta));
                            }
                            AgentEvent::ReasoningDelta { delta, .. } => {
                                // ReasoningDelta 也是文本的一部分，像 TextDelta 一样处理
                                let _ = event_tx.send(TuiEvent::AgentText(delta));
                            }
                            AgentEvent::ToolCallStart { name, .. } => {
                                let _ = event_tx.send(TuiEvent::AgentToolCall {
                                    name: name.clone(),
                                    input: serde_json::json!({}),
                                });
                            }
                            AgentEvent::ToolCallDone { .. } => {
                                let _ = event_tx.send(TuiEvent::AgentToolDone {
                                    name: "tool".to_string(),
                                });
                            }
                            AgentEvent::Error { message, .. } => {
                                let _ = event_tx.send(TuiEvent::AgentError(message));
                            }
                            _ => {
                                // Ignore other events
                            }
                        }
                    }

                    // Send response complete event
                    let _ = event_tx.send(TuiEvent::AgentResponseComplete);
                }
                Err(e) => {
                    let _ = event_tx.send(TuiEvent::AgentError(e.to_string()));
                }
            }
        });

        self.status.set_streaming(true);
    }

    /// Draw the UI
    fn draw(&mut self, frame: &mut Frame) {
        let size = frame.size();
        let areas = calculate_layout(size);

        // Draw title bar
        self.draw_title_bar(frame, areas.title);

        // Draw conversation
        self.draw_conversation(frame, areas.conversation);

        // Draw input
        self.input.render(frame, areas.input);

        // Draw input status indicator
        self.input_status.render(frame, areas.input_status);

        // Draw status bar
        self.draw_status_bar(frame, areas.status);
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
        let mut lines = Vec::new();

        for msg in &self.messages {
            match msg {
                ChatMessage::User { content } => {
                    lines.push(Line::from(vec![
                        Span::styled("You: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(content, Style::default().fg(Color::White)),
                    ]));
                }
                ChatMessage::Assistant { content } => {
                    lines.push(Line::from(vec![
                        Span::styled("Agent: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled(content, Style::default().fg(Color::White)),
                    ]));
                }
                ChatMessage::ToolCall { name, status } => {
                    let status_icon = match status {
                        ToolStatus::Running => "⏳",
                        ToolStatus::Done => "✓",
                        ToolStatus::Error(_) => "✗",
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(format!("[{}] Tool: {}", status_icon, name), Style::default().fg(Color::Yellow)),
                    ]));
                }
                ChatMessage::System { content } => {
                    lines.push(Line::from(vec![
                        Span::styled("System: ", Style::default().fg(Color::Gray)),
                        Span::styled(content, Style::default().fg(Color::Gray)),
                    ]));
                }
            }
        }

        // Add current streaming response
        if !self.current_response.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Agent: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(&self.current_response, Style::default().fg(Color::White)),
                Span::styled("▌", Style::default().fg(Color::DarkGray)), // Cursor
            ]));
        }

        // Use Paragraph with wrap enabled for automatic text wrapping
        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Conversation"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Draw the status bar
    fn draw_status_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let help_text = "Enter:发送 ↑↓:滚动 ESC:退出";
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
}
