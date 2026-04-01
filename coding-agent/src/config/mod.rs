//! Configuration module
//!
//! Handles loading, merging, and building configuration from multiple sources.

mod paths;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use genai::Client;
use serde::{Deserialize, Serialize};
use tirea_agentos::{AgentDefinition, AgentOs, AgentOsBuilder};
use tirea::prelude::Tool;

pub use paths::ConfigPaths;

/// Default system prompt file path (backward compatible)
const DEFAULT_PROMPT_PATH: &str = "config/prompt.txt";

/// Load system prompt from the external file
fn load_system_prompt() -> Result<String> {
    // Try multiple locations for the prompt file
    let candidates = ConfigPaths::discover().prompt_candidates();

    for path in candidates {
        if path.exists() {
            return std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read system prompt file: {}", path.display()));
        }
    }

    // Fallback
    let default = Path::new(DEFAULT_PROMPT_PATH);
    std::fs::read_to_string(default)
        .with_context(|| format!("Failed to read system prompt file: {}", default.display()))
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub endpoint: String,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    #[serde(default)]
    pub adapter_kind: Option<String>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthConfig {
    Env { name: String },
    Token { value: String },
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntryConfig {
    pub id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub max_rounds: Option<usize>,
}

/// Full configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub models: HashMap<String, ModelConfig>,
    #[serde(default)]
    pub agents: Vec<AgentEntryConfig>,
}

/// Load configuration from a JSON file if it exists
fn load_config_file_if_exists(path: &Path) -> Result<Option<Config>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(Some(config))
}

/// Merge two configs: `base` is the foundation, `override_config` takes precedence.
fn merge_config(base: Config, override_config: Config) -> Config {
    let mut merged = base;

    // Merge providers: override_config wins for overlapping keys
    for (key, value) in override_config.providers {
        merged.providers.insert(key, value);
    }

    // Merge models
    for (key, value) in override_config.models {
        merged.models.insert(key, value);
    }

    // Merge agents: override_config's agents replace base's by id
    let mut agent_map: HashMap<String, AgentEntryConfig> = merged
        .agents
        .into_iter()
        .map(|a| (a.id.clone(), a))
        .collect();

    for agent in override_config.agents {
        agent_map.insert(agent.id.clone(), agent);
    }

    merged.agents = agent_map.into_values().collect();
    merged
}

/// Load configuration with layered merging.
///
/// Priority (highest wins):
/// 1. Project config (`./config/agent.json` - backward compatible)
/// 2. Project-level config (`./.coding-agent/config.json`)
/// 3. User config (`~/.coding-agent/config.json`)
/// 4. Environment variable fallback
pub fn load_config_or_default() -> Result<Config> {
    let paths = ConfigPaths::discover();

    // Load from all config layers
    let has_user = paths.user_config.exists();
    let has_project = paths.project_config.exists();
    let has_local = paths.local_config.exists();

    let user = load_config_file_if_exists(&paths.user_config)?;
    let project = load_config_file_if_exists(&paths.project_config)?;
    let local = load_config_file_if_exists(&paths.local_config)?;

    // Merge: user < project < local
    let base = user.unwrap_or_default();
    let merged = match project {
        Some(proj) => merge_config(base, proj),
        None => base,
    };
    let merged = match local {
        Some(loc) => merge_config(merged, loc),
        None => merged,
    };

    // If no config files were found, fall back to environment variables
    if !has_user && !has_project && !has_local {
        return Ok(build_config_from_env());
    }

    // Inject system prompt for all agents
    let mut config = merged;
    let system_prompt = load_system_prompt()
        .unwrap_or_else(|_| "You are a helpful coding assistant.".to_string());
    for agent in &mut config.agents {
        agent.system_prompt = system_prompt.clone();
    }

    Ok(config)
}

/// Build a minimal config from environment variables
fn build_config_from_env() -> Config {
    let model = std::env::var("AGENT_MODEL").unwrap_or_else(|_| "glm-4.7".to_string());

    let endpoint = std::env::var("OPENAI_BASE_URL")
        .or_else(|_| std::env::var("ANTHROPIC_BASE_URL"))
        .unwrap_or_else(|_| "https://open.bigmodel.cn/api/coding/paas/v4/".to_string());

    let adapter_kind = if model.starts_with("glm") {
        Some("openai".to_string())
    } else {
        None
    };

    Config {
        providers: {
            let mut map = HashMap::new();
            map.insert("default".to_string(), ProviderConfig {
                endpoint,
                auth: Some(AuthConfig::Env { name: "OPENAI_API_KEY".to_string() }),
                adapter_kind,
            });
            map
        },
        models: {
            let mut map = HashMap::new();
            map.insert("default".to_string(), ModelConfig {
                provider: "default".to_string(),
                model,
            });
            map
        },
        agents: vec![AgentEntryConfig {
            id: "coding-agent".to_string(),
            model: Some("default".to_string()),
            system_prompt: load_system_prompt()
                .unwrap_or_else(|_| "You are a helpful coding assistant.".to_string()),
            max_rounds: Some(50),
        }],
    }
}

/// Create a genai client from provider configuration
pub fn create_client_from_config(config: &ProviderConfig) -> Result<Client> {
    let endpoint = if config.endpoint.ends_with('/') {
        config.endpoint.clone()
    } else {
        format!("{}/", config.endpoint)
    };
    let auth = match &config.auth {
        None => genai::resolver::AuthData::None,
        Some(AuthConfig::Env { name }) => {
            let api_key = std::env::var(name)
                .with_context(|| format!("API key not found: {}", name))?;
            genai::resolver::AuthData::from_single(api_key)
        }
        Some(AuthConfig::Token { value }) => {
            genai::resolver::AuthData::from_single(value.clone())
        }
    };
    let adapter_kind = config.adapter_kind.clone();

    let client = Client::builder()
        .with_service_target_resolver_fn(move |mut t: genai::ServiceTarget| {
            t.endpoint = genai::resolver::Endpoint::from_owned(&*endpoint);
            t.auth = auth.clone();
            if let Some(kind) = adapter_kind.as_deref() {
                if let Some(parsed_kind) = genai::adapter::AdapterKind::from_lower_str(kind) {
                    t.model = genai::ModelIden::new(parsed_kind, t.model.model_name.clone());
                }
            }
            Ok(t)
        })
        .build();

    Ok(client)
}

/// Build an AgentOs instance from configuration and tools
pub fn build_agent_os_from_config(
    config: &Config,
    tools: HashMap<String, Arc<dyn Tool>>,
) -> Result<AgentOs> {
    let mut builder = AgentOsBuilder::new();

    // Add providers
    for (provider_id, provider_config) in &config.providers {
        let client = create_client_from_config(provider_config)
            .with_context(|| format!("Failed to create client for provider: {}", provider_id))?;
        builder = builder.with_provider(provider_id, client);
    }

    // Add models
    for (model_id, model_config) in &config.models {
        let model_def = tirea_agentos::composition::ModelDefinition::new(
            model_config.provider.clone(),
            model_config.model.clone(),
        );
        builder = builder.with_model(model_id, model_def);
    }

    // Add tools
    builder = builder.with_tools(tools);

    // Add agents
    for agent_config in &config.agents {
        let mut agent_def = AgentDefinition::new(agent_config.model.clone().unwrap_or_default());
        agent_def.id = agent_config.id.clone();
        agent_def.system_prompt = agent_config.system_prompt.clone();
        if let Some(max_rounds) = agent_config.max_rounds {
            agent_def.max_rounds = max_rounds;
        }
        builder = builder.with_agent_spec(tirea_agentos::composition::AgentDefinitionSpec::local_with_id(&agent_config.id, agent_def));
    }

    // Add state store
    let sessions_dir = std::path::Path::new("./sessions");
    std::fs::create_dir_all(sessions_dir)
        .context("Failed to create sessions directory")?;
    let file_store = Arc::new(tirea_store_adapters::FileStore::new(sessions_dir)) as Arc<dyn tirea_contract::storage::ThreadStore>;
    builder = builder.with_agent_state_store(file_store);

    let agent_os = builder.build()
        .context("Failed to build AgentOs")?;

    Ok(agent_os)
}

/// Load configuration and build AgentOs (convenience function)
pub fn load_and_build_agent_os(
    tools: HashMap<String, Arc<dyn Tool>>,
) -> Result<AgentOs> {
    let config = load_config_or_default()
        .context("Failed to load configuration")?;

    build_agent_os_from_config(&config, tools)
}

/// Load agent configuration from a JSON file
pub fn load_config_file(path: impl AsRef<Path>) -> Result<Config> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let mut config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    let system_prompt = load_system_prompt()
        .unwrap_or_else(|_| "You are a helpful coding assistant.".to_string());
    for agent in &mut config.agents {
        agent.system_prompt = system_prompt.clone();
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let config_json = serde_json::json!({
            "providers": {
                "bigmodel-coding": {
                    "endpoint": "https://open.bigmodel.cn/api/coding/paas/v4/",
                    "auth": { "kind": "env", "name": "OPENAI_API_KEY" },
                    "adapter_kind": "openai"
                }
            },
            "models": {
                "glm": { "provider": "bigmodel-coding", "model": "GLM-4.5-air" }
            },
            "agents": [{
                "id": "coding-agent",
                "model": "glm",
                "system_prompt": "Test prompt",
                "max_rounds": 50
            }]
        });

        let config: Config = serde_json::from_value(config_json).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.models.len(), 1);
        assert_eq!(config.agents.len(), 1);
    }

    #[test]
    fn test_merge_config() {
        let base = Config {
            providers: {
                let mut m = HashMap::new();
                m.insert("a".to_string(), ProviderConfig {
                    endpoint: "http://base".to_string(),
                    auth: None,
                    adapter_kind: None,
                });
                m
            },
            ..Default::default()
        };

        let override_cfg = Config {
            providers: {
                let mut m = HashMap::new();
                m.insert("b".to_string(), ProviderConfig {
                    endpoint: "http://override".to_string(),
                    auth: None,
                    adapter_kind: None,
                });
                m
            },
            ..Default::default()
        };

        let merged = merge_config(base, override_cfg);
        assert_eq!(merged.providers.len(), 2);
        assert_eq!(merged.providers["a"].endpoint, "http://base");
        assert_eq!(merged.providers["b"].endpoint, "http://override");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.providers.is_empty());
        assert!(config.models.is_empty());
        assert!(config.agents.is_empty());
    }

    #[test]
    fn test_load_nonexistent_config() {
        let result = load_config_file_if_exists(Path::new("/nonexistent/path.json"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
