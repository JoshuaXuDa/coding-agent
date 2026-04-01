//! Tirea-based QueryEngine implementation
//!
//! Wraps tirea's AgentOs to implement the QueryEngine trait,
//! handling RunRequest construction, streaming, and event translation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use tirea::contracts::AgentEvent;
use tirea::prelude::Message;
use tirea_agentos::AgentOs;

use super::cancellation::CancellationToken;
use super::engine::{QueryEngine, QueryEvent, QueryRequest};
use tokio::sync::mpsc;

/// TireaQueryEngine wraps an AgentOs and implements the QueryEngine trait.
///
/// It translates tirea's AgentEvent stream into the UI-agnostic QueryEvent
/// types, keeping the TUI completely decoupled from tirea internals.
pub struct TireaQueryEngine {
    agent_os: Arc<AgentOs>,
    running: Arc<AtomicBool>,
    cancel_token: CancellationToken,
}

impl TireaQueryEngine {
    /// Create a new TireaQueryEngine wrapping the given AgentOs.
    pub fn new(agent_os: AgentOs) -> Self {
        Self {
            agent_os: Arc::new(agent_os),
            running: Arc::new(AtomicBool::new(false)),
            cancel_token: CancellationToken::new(),
        }
    }

    /// Send a log entry via the log channel.
    fn send_log(
        log_tx: &mpsc::UnboundedSender<crate::logging::LogEntry>,
        level: log::Level,
        message: impl Into<String>,
    ) {
        let _ = log_tx.send(crate::logging::LogEntry {
            level,
            module: Some("query-engine".to_string()),
            message: message.into(),
            timestamp: chrono::Local::now(),
        });
    }

    /// Translate an AgentEvent into a QueryEvent and send it.
    fn translate_and_send(
        event: AgentEvent,
        event_tx: &mpsc::UnboundedSender<QueryEvent>,
        log_tx: &mpsc::UnboundedSender<crate::logging::LogEntry>,
        event_count: u32,
    ) {
        match event {
            AgentEvent::TextDelta { delta, .. } => {
                Self::send_log(
                    log_tx,
                    log::Level::Debug,
                    format!("Event #{} TextDelta: {} chars", event_count, delta.len()),
                );
                let _ = event_tx.send(QueryEvent::TextDelta(delta));
            }
            AgentEvent::ReasoningDelta { delta, .. } => {
                Self::send_log(
                    log_tx,
                    log::Level::Debug,
                    format!(
                        "Event #{} ReasoningDelta: {} chars",
                        event_count,
                        delta.len()
                    ),
                );
                let _ = event_tx.send(QueryEvent::ReasoningDelta(delta));
            }
            AgentEvent::ToolCallStart { name, .. } => {
                Self::send_log(
                    log_tx,
                    log::Level::Info,
                    format!("Event #{} ToolCallStart: {}", event_count, name),
                );
                let _ = event_tx.send(QueryEvent::ToolCallStart {
                    name: name.clone(),
                    input: serde_json::json!({}),
                });
            }
            AgentEvent::ToolCallDone { id, .. } => {
                Self::send_log(
                    log_tx,
                    log::Level::Info,
                    format!("Event #{} ToolCallDone: {}", event_count, id),
                );
                let _ = event_tx.send(QueryEvent::ToolCallDone { name: id });
            }
            AgentEvent::Error { message, .. } => {
                Self::send_log(
                    log_tx,
                    log::Level::Error,
                    format!("Event #{} AgentError: {}", event_count, message),
                );
                let _ = event_tx.send(QueryEvent::Error(message));
            }
            _ => {
                Self::send_log(
                    log_tx,
                    log::Level::Debug,
                    format!(
                        "Event #{} Other: {:?}",
                        event_count,
                        std::mem::discriminant(&event)
                    ),
                );
            }
        }
    }
}

impl QueryEngine for TireaQueryEngine {
    fn submit(
        &self,
        request: QueryRequest,
        event_tx: mpsc::UnboundedSender<QueryEvent>,
    ) {
        // Set running state, reset cancellation
        self.running.store(true, Ordering::SeqCst);
        self.cancel_token.reset();

        let log_tx = request.log_tx.clone();
        let message = request.message.clone();
        let agent_os = self.agent_os.clone();
        let running = self.running.clone();
        let cancel_token = self.cancel_token.clone();

        Self::send_log(
            &log_tx,
            log::Level::Info,
            format!("Starting agent task for message: {}", message),
        );

        tokio::spawn(async move {
            let run_request = tirea_contract::RunRequest {
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
                source_mailbox_entry_id: None,
            };

            Self::send_log(
                &log_tx,
                log::Level::Info,
                format!(
                    "Calling agent_os.run_stream() with agent_id: {}",
                    run_request.agent_id
                ),
            );

            match agent_os.run_stream(run_request).await {
                Ok(mut stream) => {
                    Self::send_log(
                        &log_tx,
                        log::Level::Info,
                        "Agent stream created successfully, waiting for events...",
                    );

                    let mut event_count = 0u32;

                    while let Some(event) = stream.events.next().await {
                        // Check for cancellation
                        if cancel_token.is_cancelled() {
                            Self::send_log(&log_tx, log::Level::Info, "Query cancelled by user");
                            let _ = event_tx.send(QueryEvent::Error("Cancelled".into()));
                            break;
                        }

                        event_count += 1;
                        Self::translate_and_send(event, &event_tx, &log_tx, event_count);
                    }

                    Self::send_log(
                        &log_tx,
                        log::Level::Info,
                        format!(
                            "Agent stream ended. Total events received: {}",
                            event_count
                        ),
                    );

                    // Send completion event (unless cancelled)
                    if !cancel_token.is_cancelled() {
                        let _ = event_tx.send(QueryEvent::Complete);
                    }
                }
                Err(e) => {
                    Self::send_log(
                        &log_tx,
                        log::Level::Error,
                        format!("Failed to create agent stream: {}", e),
                    );
                    let _ = event_tx.send(QueryEvent::Error(e.to_string()));
                }
            }

            Self::send_log(&log_tx, log::Level::Info, "Agent task completed");
            running.store(false, Ordering::SeqCst);
        });
    }

    fn cancel(&self) {
        self.cancel_token.cancel();
    }

    fn is_busy(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
