//! Interactive prompt for file selection

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// Interactive file selector
pub struct FileSelector;

impl FileSelector {
    /// Display candidates and let user select a file
    pub fn select_from_candidates(candidates: &[PathBuf]) -> Result<Option<usize>, anyhow::Error> {
        println!("\n找到 {} 个匹配文件:", candidates.len());

        for (i, file) in candidates.iter().enumerate() {
            println!("  [{}] {}", i + 1, file.display());
        }

        println!("\n选择文件 (1-{}), 或按 Enter 跳过:", candidates.len());

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print("> ");
            stdout.flush()?;

            let mut input = String::new();
            stdin.lock().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                return Ok(None);
            }

            if let Ok(num) = input.parse::<usize>() {
                if num >= 1 && num <= candidates.len() {
                    return Ok(Some(num - 1));
                }
            }

            println!("⚠️  无效选择，请输入 1-{} 之间的数字", candidates.len());
        }
    }

    /// Confirm with user (yes/no)
    pub fn confirm(message: &str) -> Result<bool, anyhow::Error> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("{} (y/n): ", message);
            stdout.flush()?;

            let mut input = String::new();
            stdin.lock().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => {
                    println!("⚠️  请输入 y 或 n");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_selector_exists() {
        // Verify the type exists
        let _: FileSelector;
    }
}
