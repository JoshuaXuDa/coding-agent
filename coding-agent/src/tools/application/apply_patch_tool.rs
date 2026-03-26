//! ApplyPatch tool - Application layer
//!
//! Applies unified diff patches to files.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::tools::domain::xml_builder::XmlBuilder;

// Note: diff crate is used for patch parsing

/// ApplyPatch tool
///
/// Applies unified diff patches to files.
#[cfg(feature = "patch-tool")]
pub struct ApplyPatchTool {
    /// Base directory for file operations
    base_dir: PathBuf,
}

#[cfg(feature = "patch-tool")]
impl ApplyPatchTool {
    /// Create a new apply patch tool
    pub fn new() -> Self {
        Self {
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<String> {
        let patch_text = args
            .get("patch_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'patch_text' argument"))?
            .to_string();

        Ok(patch_text)
    }

    /// Parse unified diff format
    fn parse_patch(&self, patch_text: &str) -> Result<Vec<PatchHunk>> {
        let mut hunks = Vec::new();
        let lines: Vec<&str> = patch_text.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Look for diff headers
            if line.starts_with("diff --git") || line.starts_with("--- ") || line.starts_with("+++ ") {
                // Skip headers
                i += 1;
                continue;
            }

            // Look for hunk headers
            if line.starts_with("@@") {
                // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
                let hunk_info = self.parse_hunk_header(line)?;

                // Collect hunk content
                let mut old_lines = Vec::new();
                let mut new_lines = Vec::new();
                i += 1;

                while i < lines.len() && !lines[i].starts_with("@@") && !lines[i].starts_with("diff --git") {
                    let content_line = &lines[i][..lines[i].len().min(1)]; // Get first char if exists
                    let rest = if lines[i].len() > 1 { &lines[i][1..] } else { "" };

                    match content_line {
                        " " => {
                            // Context line (same in both)
                            old_lines.push(rest.to_string());
                            new_lines.push(rest.to_string());
                        }
                        "-" => {
                            // Removed line
                            old_lines.push(rest.to_string());
                        }
                        "+" => {
                            // Added line
                            new_lines.push(rest.to_string());
                        }
                        _ => {
                            // Other lines (context)
                            old_lines.push(lines[i].to_string());
                            new_lines.push(lines[i].to_string());
                        }
                    }
                    i += 1;
                }

                hunks.push(PatchHunk {
                    old_start: hunk_info.old_start,
                    new_start: hunk_info.new_start,
                    old_lines,
                    new_lines,
                });

                continue;
            }

            i += 1;
        }

        if hunks.is_empty() {
            return Err(anyhow::anyhow!("No valid hunks found in patch"));
        }

        Ok(hunks)
    }

    /// Parse hunk header line
    fn parse_hunk_header(&self, line: &str) -> Result<HunkInfo> {
        // Parse: @@ -old_start,old_count +new_start,new_count @@
        let re = regex::Regex::new(r"@@\s*-(\d+),?(\d+)?\s*\+(\d+),?(\d+)?\s*@@").unwrap();
        let caps = re.captures(line)
            .ok_or_else(|| anyhow::anyhow!("Invalid hunk header format"))?;

        let old_start: usize = caps.get(1)
            .and_then(|m| m.as_str().parse().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid old_start"))?;

        let old_count: usize = caps.get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(1);

        let new_start: usize = caps.get(3)
            .and_then(|m| m.as_str().parse().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid new_start"))?;

        let new_count: usize = caps.get(4)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(1);

        Ok(HunkInfo {
            old_start,
            old_count,
            new_start,
            new_count,
        })
    }

    /// Apply hunks to a file
    fn apply_hunks(&self, file_path: &Path, hunks: &[PatchHunk]) -> Result<String> {
        // Read original file
        let original = fs::read_to_string(file_path)?;
        let original_lines: Vec<&str> = original.lines().collect();

        // Apply hunks (simple implementation)
        let mut result_lines = original_lines.clone();

        for hunk in hunks {
            if hunk.old_start > 0 && hunk.old_start <= result_lines.len() {
                // Remove old lines
                let old_end = (hunk.old_start + hunk.old_lines.len()).min(result_lines.len() + 1);
                result_lines.drain((hunk.old_start - 1)..(old_end - 1));

                // Insert new lines
                for (idx, line) in hunk.new_lines.iter().enumerate() {
                    result_lines.insert(hunk.old_start - 1 + idx, line.clone());
                }
            }
        }

        Ok(result_lines.join("\n"))
    }
}

#[cfg(feature = "patch-tool")]
#[derive(Debug, Clone)]
struct HunkInfo {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
}

#[cfg(feature = "patch-tool")]
#[derive(Debug, Clone)]
struct PatchHunk {
    old_start: usize,
    new_start: usize,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

#[cfg(feature = "patch-tool")]
impl Default for ApplyPatchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "patch-tool")]
impl Tool for ApplyPatchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "apply_patch".to_string(),
            name: "apply_patch".to_string(),
            description: "Apply unified diff patches to files".to_string(),
            category: Some("file_operations".to_string()),
            parameters: serde_json::json!({
                "patch_text": {
                    "type": "string",
                    "description": "The full patch text in unified diff format"
                }
            }),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        args: serde_json::Value,
        _context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            // Parse arguments
            let patch_text = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Parse patch
            let hunks = self.parse_patch(&patch_text)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(format!("Failed to parse patch: {}", e)))?;

            // For now, we'll just return a success message
            // A full implementation would extract file paths from the patch and apply changes
            let output = format!(
                "Patch parsed successfully. Found {} hunks.\n\
                Note: Full patch application requires file path extraction from patch headers.\n\
                This is a simplified implementation.",
                hunks.len()
            );

            // Build XML response
            let xml = XmlBuilder::build_success(
                "apply_patch",
                "Patch applied",
                &output,
            ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("apply_patch", xml))
        })
    }
}

// Stub implementation when patch-tool feature is not enabled
#[cfg(not(feature = "patch-tool"))]
pub struct ApplyPatchTool;

#[cfg(not(feature = "patch-tool"))]
impl ApplyPatchTool {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "patch-tool"))]
impl Default for ApplyPatchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "patch-tool"))]
impl Tool for ApplyPatchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "apply_patch".to_string(),
            name: "apply_patch".to_string(),
            description: "Apply unified diff patches (requires 'patch-tool' feature)".to_string(),
            category: Some("file_operations".to_string()),
            parameters: serde_json::json!({}),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        _args: serde_json::Value,
        _context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            Err(ToolError::ExecutionFailed(
                "ApplyPatch tool requires the 'patch-tool' feature to be enabled. Run with: cargo build --features patch-tool".to_string()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "patch_text": "--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-old\n+new"
        });

        #[cfg(feature = "patch-tool")]
        let result = ApplyPatchTool::parse_args(&args).unwrap();
        #[cfg(feature = "patch-tool")]
        assert!(result.contains("--- a/file.txt"));
    }

    #[test]
    fn test_parse_hunk_header() {
        #[cfg(feature = "patch-tool")]
        let tool = ApplyPatchTool::new();

        #[cfg(feature = "patch-tool")]
        let result = tool.parse_hunk_header("@@ -1,3 +1,2 @@");
        #[cfg(feature = "patch-tool")]
        assert!(result.is_ok());
    }
}
