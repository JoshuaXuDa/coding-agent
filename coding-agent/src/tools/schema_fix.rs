//! Schema fixing wrapper for Anthropic API compatibility.
//!
//! Anthropic's API requires `input_schema` to be a valid JSON Schema with
//! `{"type": "object", "properties": {...}}`, but our tool descriptors define
//! `parameters` as a flat object of property schemas. This wrapper transparently
//! normalizes the schema at the descriptor level.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use serde_json::{Value, json};
use tirea::prelude::Tool;
use tirea_contract::runtime::tool_call::{ToolCallContext, ToolDescriptor, ToolError, ToolResult};

/// Wrap a tool's parameters into a proper JSON Schema if needed.
///
/// - If the parameters already have `"type": "object"`, returns as-is.
/// - Otherwise, wraps into `{"type": "object", "properties": <original>, "required": <keys>}`.
pub fn normalize_parameters(params: Value) -> Value {
    if params.get("type").and_then(|v| v.as_str()) == Some("object") {
        return params;
    }

    if let Some(obj) = params.as_object() {
        let required: Vec<&str> = obj
            .iter()
            .filter(|(_, v)| {
                // Mark as required unless it has a default value
                v.get("default").is_none()
            })
            .map(|(k, _)| k.as_str())
            .collect();

        let mut schema = json!({
            "type": "object",
            "properties": params,
        });
        if !required.is_empty() {
            schema["required"] = json!(required);
        }
        schema
    } else {
        params
    }
}

/// A tool wrapper that normalizes the parameter schema for Anthropic compatibility.
pub struct SchemaFixingTool {
    inner: Arc<dyn Tool>,
}

impl SchemaFixingTool {
    pub fn new(inner: Arc<dyn Tool>) -> Self {
        Self { inner }
    }
}

impl Tool for SchemaFixingTool {
    fn descriptor(&self) -> ToolDescriptor {
        let mut desc = self.inner.descriptor();
        desc.parameters = normalize_parameters(desc.parameters);
        desc
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        args: Value,
        ctx: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        self.inner.execute(args, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_already_valid_schema() {
        let params = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let result = normalize_parameters(params);
        assert_eq!(result["type"], "object");
        assert!(result.get("required").is_none());
    }

    #[test]
    fn test_flat_schema_wrapped() {
        let params = json!({
            "command": {"type": "string", "description": "Command to run"},
            "timeout": {"type": "number", "default": 30}
        });
        let result = normalize_parameters(params);
        assert_eq!(result["type"], "object");
        assert!(result["properties"]["command"].is_object());
        assert!(result["properties"]["timeout"].is_object());
        // timeout has a default, so it should not be required
        assert_eq!(result["required"], json!(["command"]));
    }

    #[test]
    fn test_all_required() {
        let params = json!({
            "path": {"type": "string"},
            "pattern": {"type": "string"}
        });
        let result = normalize_parameters(params);
        let required = result["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
    }

    #[test]
    fn test_empty_schema() {
        let params = json!({});
        let result = normalize_parameters(params);
        assert_eq!(result["type"], "object");
        assert_eq!(result["properties"], json!({}));
        assert!(result.get("required").is_none());
    }
}
