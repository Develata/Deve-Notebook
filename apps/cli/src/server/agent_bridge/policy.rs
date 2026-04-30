//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use deve_core::config::Config;
use serde::Serialize;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AgentBridgePolicy {
    enabled: bool,
    trusted: bool,
    native_enabled: bool,
    requested_mode: String,
    cli_path: Option<String>,
    timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentBridgeCapabilities {
    pub native_available: bool,
    pub native_reason: Option<String>,
    pub trusted_cli_available: bool,
    pub trusted_cli_reason: Option<String>,
    pub effective_backend: String,
    pub effective_backend_reason: Option<String>,
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
            native_enabled: config.ai.native_enabled,
            requested_mode: config.ai.mode.clone(),
            cli_path: std::env::var("AGENT_CLI_PATH").ok().and_then(non_empty),
            timeout_ms: config.ai.agent_bridge.timeout_ms,
        }
    }

    pub fn capabilities(&self) -> AgentBridgeCapabilities {
        let trusted_cli_reason = self.spawn_path().err();
        let trusted_cli_available = trusted_cli_reason.is_none();
        let native_reason =
            (!self.native_enabled).then(|| "native AI disabled by config".to_string());
        let (effective_backend, effective_backend_reason) = if self.requested_mode == "trusted-cli"
        {
            if trusted_cli_available {
                ("trusted-cli".to_string(), None)
            } else if self.native_enabled {
                ("native".to_string(), trusted_cli_reason.clone())
            } else {
                (
                    "none".to_string(),
                    Some(
                        trusted_cli_reason
                            .clone()
                            .unwrap_or_else(|| "no AI backend available".to_string()),
                    ),
                )
            }
        } else if self.native_enabled {
            ("native".to_string(), None)
        } else if trusted_cli_available {
            (
                "trusted-cli".to_string(),
                Some("native AI disabled by config".to_string()),
            )
        } else {
            (
                "none".to_string(),
                Some("no AI backend available".to_string()),
            )
        };

        AgentBridgeCapabilities {
            native_available: self.native_enabled,
            native_reason,
            trusted_cli_available,
            trusted_cli_reason,
            effective_backend,
            effective_backend_reason,
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
        if !is_executable_file(Path::new(&path)) {
            return Err("AGENT_CLI_PATH must point to an executable file".to_string());
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

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
#[path = "policy_test.rs"]
mod tests;
