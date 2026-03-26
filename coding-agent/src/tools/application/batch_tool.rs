//! Batch tool - Application layer
//!
//! Executes multiple tool calls in parallel for improved performance.

use anyhow::Result;
use std::pin::Pin;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::tools::domain::xml_builder::XmlBuilder;

/// Disallowed tools in batch mode
const DISALLOWED_TOOLS: &[&str] = &["batch"];

/// Maximum number of tools in a single batch
const MAX_BATCH_SIZE: usize = 25;

/// Batch tool
///
/// Executes multiple tool calls in parallel.
pub struct BatchTool;

impl BatchTool {
    /// Create a new batch tool
    pub fn new() -> Self {
        Self
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<Vec<ToolCall>> {
        let tool_calls_array = args
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing 'tool_calls' argument"))?;

        if tool_calls_array.is_empty() {
            return Err(anyhow::anyhow!("tool_calls must contain at least one tool call"));
        }

        let mut tool_calls = Vec::new();
        for (idx, call) in tool_calls_array.iter().enumerate() {
            let tool = call
                .get("tool")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Tool call {} missing 'tool' field", idx))?
                .to_string();

            let parameters = call
                .get("parameters")
                .cloned()
                .unwrap_or(serde_json::Value::Object(Default::default()));

            tool_calls.push(ToolCall { tool, parameters });
        }

        Ok(tool_calls)
    }

    /// Execute a single tool call
    async fn execute_tool_call(
        &self,
        call: &ToolCall,
        context: &ToolCallContext<'_>,
    ) -> BatchResult {
        let start_time = std::time::Instant::now();

        // Check if tool is disallowed
        if DISALLOWED_TOOLS.contains(&call.tool.as_str()) {
            return BatchResult {
                tool: call.tool.clone(),
                success: false,
                error: Some(format!(
                    "Tool '{}' is not allowed in batch. Disallowed tools: {}",
                    call.tool,
                    DISALLOWED_TOOLS.join(", ")
                )),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
        }

        // Get the global tool registry
        let tools = match crate::tools::get_tool_registry() {
            Some(t) => t,
            None => {
                return BatchResult {
                    tool: call.tool.clone(),
                    success: false,
                    error: Some("Tool registry not initialized".to_string()),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        // Get the tool from registry
        let tool = match tools.get(&call.tool) {
            Some(t) => t,
            None => {
                return BatchResult {
                    tool: call.tool.clone(),
                    success: false,
                    error: Some(format!(
                        "Tool '{}' not found in registry",
                        call.tool
                    )),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        // Execute the tool
        match tool.execute(call.parameters.clone(), context).await {
            Ok(_) => BatchResult {
                tool: call.tool.clone(),
                success: true,
                error: None,
                duration_ms: start_time.elapsed().as_millis() as u64,
            },
            Err(e) => BatchResult {
                tool: call.tool.clone(),
                success: false,
                error: Some(e.to_string()),
                duration_ms: start_time.elapsed().as_millis() as u64,
            },
        }
    }
}

/// Tool call in batch
#[derive(Debug, Clone)]
struct ToolCall {
    tool: String,
    parameters: serde_json::Value,
}

/// Result of a single tool execution in batch
#[derive(Debug, Clone)]
struct BatchResult {
    tool: String,
    success: bool,
    error: Option<String>,
    duration_ms: u64,
}

impl Default for BatchTool {
    fn default() -> Self {
        Self
    }
}

impl Tool for BatchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "batch".to_string(),
            name: "batch".to_string(),
            description: "Execute multiple tool calls in parallel for improved performance".to_string(),
            category: Some("execution".to_string()),
            parameters: serde_json::json!({
                "tool_calls": {
                    "type": "array",
                    "description": "Array of tool calls to execute in parallel",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": {
                                "type": "string",
                                "description": "The name of the tool to execute"
                            },
                            "parameters": {
                                "type": "object",
                                "description": "Parameters for the tool"
                            }
                        },
                        "required": ["tool"]
                    },
                    "minItems": 1
                }
            }),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        args: serde_json::Value,
        context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            // Parse arguments
            let mut tool_calls = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Limit batch size
            let discarded_calls = if tool_calls.len() > MAX_BATCH_SIZE {
                tool_calls.split_off(MAX_BATCH_SIZE)
            } else {
                Vec::new()
            };

            // Execute all tool calls in parallel
            let mut futures = Vec::new();
            for call in &tool_calls {
                futures.push(self.execute_tool_call(call, context));
            }

            // Wait for all executions to complete
            let results = futures::future::join_all(futures).await;

            // Add discarded calls as errors
            let mut all_results = results;
            for call in discarded_calls {
                all_results.push(BatchResult {
                    tool: call.tool,
                    success: false,
                    error: Some("Maximum of 25 tools allowed in batch".to_string()),
                    duration_ms: 0,
                });
            }

            // Calculate statistics
            let successful = all_results.iter().filter(|r| r.success).count();
            let failed = all_results.len() - successful;

            // Build output message
            let output_message = if failed > 0 {
                format!(
                    "Executed {}/{} tools successfully. {} failed.",
                    successful,
                    all_results.len(),
                    failed
                )
            } else {
                format!(
                    "All {} tools executed successfully.",
                    successful
                )
            };

            // Build details
            let details: Vec<serde_json::Value> = all_results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "tool": r.tool,
                        "success": r.success,
                        "error": r.error,
                        "duration_ms": r.duration_ms
                    })
                })
                .collect();

            let details_json = serde_json::to_string_pretty(&details)
                .unwrap_or_else(|_| "Failed to serialize details".to_string());

            // Build XML response
            let xml = XmlBuilder::build_success(
                "batch",
                &format!("Batch execution ({}/{} successful)", successful, all_results.len()),
                &format!("{}\n\nDetails:\n{}", output_message, details_json),
            ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("batch", xml))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "tool_calls": [
                {
                    "tool": "read",
                    "parameters": {"file_path": "test.txt"}
                },
                {
                    "tool": "write",
                    "parameters": {"file_path": "out.txt", "content": "hello"}
                }
            ]
        });

        let result = BatchTool::parse_args(&args).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].tool, "read");
        assert_eq!(result[1].tool, "write");
    }

    #[test]
    fn test_parse_args_empty() {
        let args = serde_json::json!({
            "tool_calls": []
        });

        let result = BatchTool::parse_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_args_missing() {
        let args = serde_json::json!({});

        let result = BatchTool::parse_args(&args);
        assert!(result.is_err());
    }
}
