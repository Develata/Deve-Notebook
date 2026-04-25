//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use deve_core::config::Config;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AgentBridgePolicy {
    enabled: bool,
    trusted: bool,
    cli_path: Option<String>,
    timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentBridgeCapabilities {
    pub trusted_cli_available: bool,
    pub trusted_cli_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentBridgeRunConfig {
    pub cli_path: String,
    pub timeout_ms: u64,
}

impl AgentBridgePolicy {
    pub fn from_config(config: &Config) -> Self {
        Self {
            enabled: env_bool("DEVE_AI_AGENT_BRIDGE_ENABLED")
                .unwrap_or(config.ai.agent_bridge.enabled),
            trusted: env_bool("DEVE_AI_AGENT_BRIDGE_TRUSTED")
                .unwrap_or(config.ai.agent_bridge.trusted),
            cli_path: std::env::var("AGENT_CLI_PATH").ok().and_then(non_empty),
            timeout_ms: config.ai.agent_bridge.timeout_ms,
        }
    }

    pub fn capabilities(&self) -> AgentBridgeCapabilities {
        AgentBridgeCapabilities {
            trusted_cli_available: self.spawn_path().is_ok(),
            trusted_cli_reason: self.spawn_path().err(),
        }
    }

    pub fn spawn_path(&self) -> Result<String, String> {
        if !self.enabled {
            return Err("external agent disabled".to_string());
        }
        if !self.trusted {
            return Err("trusted mode required".to_string());
        }
        let path = self
            .cli_path
            .clone()
            .ok_or_else(|| "AGENT_CLI_PATH required".to_string())?;
        if !Path::new(&path).is_absolute() {
            return Err("AGENT_CLI_PATH must be absolute".to_string());
        }
        Ok(path)
    }

    pub fn run_config(&self) -> Result<AgentBridgeRunConfig, String> {
        Ok(AgentBridgeRunConfig {
            cli_path: self.spawn_path()?,
            timeout_ms: self.timeout_ms.max(1),
        })
    }
}

fn env_bool(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
#[path = "policy_test.rs"]
mod tests;
