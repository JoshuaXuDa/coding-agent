//! Configuration loading for CodingAgent
//!
//! This module handles loading and parsing the JSON configuration file,
//! then builds an AgentOs instance with providers, models, and agents.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Context, Result};
use genai::Client;
use serde::{Deserialize, Serialize};
use tirea_agentos::{AgentDefinition, AgentOs, AgentOsBuilder};
use tirea::prelude::Tool;

/// Default configuration file path
const DEFAULT_CONFIG_PATH: &str = "config/agent.json";

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub models: HashMap<String, ModelConfig>,
    pub agents: Vec<AgentEntryConfig>,
}

/// Load agent configuration from a JSON file
pub fn load_config_file(path: impl AsRef<Path>) -> Result<Config> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

/// Load configuration from the default path, with fallback to environment variables
pub fn load_config_or_default() -> Result<Config> {
    // Try to load from config file first
    if Path::new(DEFAULT_CONFIG_PATH).exists() {
        return load_config_file(DEFAULT_CONFIG_PATH);
    }

    // Fallback: create a minimal config from environment variables
    let model = std::env::var("AGENT_MODEL").unwrap_or_else(|_| "glm-4.7".to_string());

    // Determine endpoint and adapter from environment
    let endpoint = std::env::var("OPENAI_BASE_URL")
        .or_else(|_| std::env::var("ANTHROPIC_BASE_URL"))
        .unwrap_or_else(|_| "https://open.bigmodel.cn/api/coding/paas/v4/".to_string());

    let adapter_kind = if model.starts_with("glm") {
        Some("openai".to_string())
    } else {
        None
    };

    Ok(Config {
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
            system_prompt: crate::prompt::SYSTEM_PROMPT.to_string(),
            max_rounds: Some(50),
        }],
    })
}

/// Create a genai client from provider configuration
pub fn create_client_from_config(config: &ProviderConfig) -> Result<Client> {
    let endpoint = config.endpoint.clone();
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
        builder = builder.with_agent(&agent_config.id, agent_def);
    }

    // Add state store - use FileStore for persistence
    let sessions_dir = std::path::Path::new("./sessions");
    std::fs::create_dir_all(sessions_dir)
        .context("Failed to create sessions directory")?;
    let file_store = Arc::new(tirea_store_adapters::FileStore::new(sessions_dir)) as Arc<dyn tirea_contract::storage::ThreadStore>;
    builder = builder.with_agent_state_store(file_store);

    // Build the AgentOs
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
}
