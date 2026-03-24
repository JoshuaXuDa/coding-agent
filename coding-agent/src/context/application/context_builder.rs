//! Context builder - orchestrates the entire context injection process

use crate::context::domain::{parser::AtSymbolParser, injector::ContextInjector};
use crate::context::application::file_search::FileSearchCoordinator;
use crate::platform::domain::filesystem::FileSystem;
use std::io::{self, Write, BufRead};
use std::sync::Arc;

/// Builds enhanced messages by parsing @ symbols and injecting file content
pub struct ContextBuilder {
    fs: Arc<dyn FileSystem>,
    interactive: bool,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new() -> Self {
        Self {
            fs: crate::platform::create_filesystem(),
            interactive: true,
        }
    }

    /// Set the filesystem to use
    pub fn with_filesystem(mut self, fs: Arc<dyn FileSystem>) -> Self {
        self.fs = fs;
        self
    }

    /// Enable or disable interactive mode
    pub fn with_interactive_mode(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Build enhanced message by parsing @ symbols and injecting file content
    pub async fn build_context(&self, user_message: &str) -> Result<String, anyhow::Error> {
        // 1. Parse @ symbols
        let references = AtSymbolParser::extract_references(user_message);

        if references.is_empty() {
            return Ok(user_message.to_string());
        }

        // 2. Resolve each reference
        let search_coordinator = FileSearchCoordinator::new(self.fs.clone());
        let injector = ContextInjector::new(self.fs.clone());

        let mut enhanced_message = user_message.to_string();
        let mut injected_contents = Vec::new();

        for reference in &references {
            let candidates = search_coordinator.resolve_reference(reference).await?;

            if candidates.is_empty() {
                eprintln!("⚠️  未找到匹配文件: {}", reference.raw_reference);
                continue;
            }

            let selected_path = if candidates.len() == 1 {
                candidates[0].clone()
            } else if self.interactive {
                // Interactive selection
                Self::select_file_interactive(&candidates)?
            } else {
                // Non-interactive mode: use first match
                candidates[0].clone()
            };

            // 3. Inject file content
            match injector.inject_file(&selected_path).await {
                Ok(content) => {
                    injected_contents.push(content);
                }
                Err(e) => {
                    eprintln!("⚠️  无法读取文件 {}: {}", selected_path.display(), e);
                }
            }
        }

        // 4. Append file content to message
        for content in injected_contents {
            enhanced_message.push_str(&ContextInjector::format_injected_content(&content));
        }

        Ok(enhanced_message)
    }

    /// Interactive file selection
    fn select_file_interactive(candidates: &[std::path::PathBuf]) -> Result<std::path::PathBuf, anyhow::Error> {
        println!("\n找到 {} 个匹配文件:", candidates.len());

        for (i, file) in candidates.iter().enumerate() {
            println!("  [{}] {}", i + 1, file.display());
        }

        println!("\n选择文件 (1-{}), 或按 Enter 跳过:", candidates.len());

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("> ");
            stdout.flush()?;

            let mut input = String::new();
            stdin.lock().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                return Err(anyhow::anyhow!("用户取消选择"));
            }

            if let Ok(num) = input.parse::<usize>() {
                if num >= 1 && num <= candidates.len() {
                    return Ok(candidates[num - 1].clone());
                }
            }

            println!("⚠️  无效选择，请输入 1-{} 之间的数字", candidates.len());
        }
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder_default() {
        let builder = ContextBuilder::default();
        assert!(builder.interactive);
    }

    #[test]
    fn test_context_builder_with_interactive_mode() {
        let builder = ContextBuilder::new().with_interactive_mode(false);
        assert!(!builder.interactive);
    }
}
