//! Configuration path discovery
//!
//! Finds configuration files across multiple locations.

use std::path::PathBuf;

/// Discovered configuration file paths.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// User-level config: `~/.coding-agent/config.json`
    pub user_config: PathBuf,
    /// Project-level config: `./.coding-agent/config.json`
    pub project_config: PathBuf,
    /// Local config (backward compatible): `./config/agent.json`
    pub local_config: PathBuf,
}

impl ConfigPaths {
    /// Discover all configuration paths based on the current environment.
    pub fn discover() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            user_config: home.join(".coding-agent").join("config.json"),
            project_config: cwd.join(".coding-agent").join("config.json"),
            local_config: cwd.join("config").join("agent.json"),
        }
    }

    /// Candidate paths for the system prompt file, in priority order.
    pub fn prompt_candidates(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        vec![
            // Local config/prompt.txt (backward compatible)
            cwd.join("config").join("prompt.txt"),
            // Project-level prompt
            cwd.join(".coding-agent").join("prompt.txt"),
            // User-level prompt
            home.join(".coding-agent").join("prompt.txt"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_paths() {
        let paths = ConfigPaths::discover();
        // Should always produce valid paths
        assert!(paths.user_config.to_string_lossy().contains(".coding-agent"));
        assert!(paths.local_config.to_string_lossy().contains("config"));
    }

    #[test]
    fn test_prompt_candidates() {
        let paths = ConfigPaths::discover();
        let candidates = paths.prompt_candidates();
        assert_eq!(candidates.len(), 3);
        // All should end with prompt.txt
        for path in &candidates {
            assert!(path.to_string_lossy().ends_with("prompt.txt"));
        }
    }
}
