use deve_core::config::Config;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct AgentBridgePolicy {
    enabled: bool,
    trusted: bool,
    cli_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentBridgeCapabilities {
    pub trusted_cli_available: bool,
    pub trusted_cli_reason: Option<String>,
}

impl AgentBridgePolicy {
    pub fn from_config(config: &Config) -> Self {
        Self {
            enabled: env_bool("DEVE_AI_AGENT_BRIDGE_ENABLED")
                .unwrap_or(config.ai.agent_bridge.enabled),
            trusted: env_bool("DEVE_AI_AGENT_BRIDGE_TRUSTED")
                .unwrap_or(config.ai.agent_bridge.trusted),
            cli_path: std::env::var("AGENT_CLI_PATH").ok().and_then(non_empty),
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
        self.cli_path
            .clone()
            .ok_or_else(|| "AGENT_CLI_PATH required".to_string())
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
mod tests {
    use super::AgentBridgePolicy;

    #[test]
    fn disabled_policy_fails_closed() {
        let policy = AgentBridgePolicy {
            enabled: false,
            trusted: false,
            cli_path: Some("agent".to_string()),
        };
        assert_eq!(
            policy.spawn_path().expect_err("must fail"),
            "external agent disabled"
        );
    }

    #[test]
    fn untrusted_policy_requires_trusted_mode() {
        let policy = AgentBridgePolicy {
            enabled: true,
            trusted: false,
            cli_path: Some("agent".to_string()),
        };
        assert_eq!(
            policy.spawn_path().expect_err("must fail"),
            "trusted mode required"
        );
    }

    #[test]
    fn trusted_policy_requires_explicit_cli_path() {
        let policy = AgentBridgePolicy {
            enabled: true,
            trusted: true,
            cli_path: None,
        };
        assert_eq!(
            policy.spawn_path().expect_err("must fail"),
            "AGENT_CLI_PATH required"
        );
    }
}
