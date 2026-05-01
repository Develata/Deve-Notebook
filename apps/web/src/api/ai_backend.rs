//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use gloo_net::http::Request;
use serde::Deserialize;

pub const AI_BACKEND_NATIVE: &str = "native";
pub const AI_BACKEND_TRUSTED_CLI: &str = "trusted-cli";
pub const AI_PLUGIN_NATIVE: &str = "ai-chat";
pub const AI_PLUGIN_TRUSTED_CLI: &str = "agent-bridge";

#[derive(Clone, Debug, Deserialize)]
pub struct AiBackendCapabilities {
    #[serde(default = "default_native_available")]
    pub native_available: bool,
    #[serde(default)]
    pub native_reason: Option<String>,
    #[serde(default)]
    pub trusted_cli_available: bool,
    #[serde(default)]
    pub trusted_cli_reason: Option<String>,
    #[serde(default = "default_effective_backend")]
    pub effective_backend: String,
    #[serde(default)]
    pub effective_backend_reason: Option<String>,
}

impl Default for AiBackendCapabilities {
    fn default() -> Self {
        Self {
            native_available: true,
            native_reason: None,
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            effective_backend: AI_BACKEND_NATIVE.to_string(),
            effective_backend_reason: None,
        }
    }
}

impl AiBackendCapabilities {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self {
            native_available: false,
            native_reason: Some(reason.clone()),
            trusted_cli_available: false,
            trusted_cli_reason: Some(reason.clone()),
            effective_backend: "none".to_string(),
            effective_backend_reason: Some(reason),
        }
    }
}

fn default_native_available() -> bool {
    true
}

fn default_effective_backend() -> String {
    AI_BACKEND_NATIVE.to_string()
}

pub async fn fetch_ai_backend_capabilities() -> AiBackendCapabilities {
    let response = Request::get("/api/ai/backend-capabilities").send().await;
    match response {
        Ok(resp) if resp.ok() => resp
            .json::<AiBackendCapabilities>()
            .await
            .unwrap_or_else(|_| {
                AiBackendCapabilities::unavailable("AI backend capability response is invalid")
            }),
        Ok(resp) => AiBackendCapabilities::unavailable(format!(
            "AI backend capability probe failed: HTTP {}",
            resp.status()
        )),
        Err(_) => AiBackendCapabilities::unavailable("AI backend capability probe failed"),
    }
}

pub fn ai_backend_to_plugin_id(backend: &str) -> &'static str {
    match backend {
        AI_BACKEND_TRUSTED_CLI => AI_PLUGIN_TRUSTED_CLI,
        _ => AI_PLUGIN_NATIVE,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendSendDecision {
    Use(&'static str),
    Switch {
        backend: &'static str,
        reason: String,
    },
    Block {
        reason: String,
    },
}

pub fn resolve_backend_for_send(
    current_backend: &str,
    cap: &AiBackendCapabilities,
) -> BackendSendDecision {
    match current_backend {
        AI_BACKEND_TRUSTED_CLI => resolve_trusted_cli(cap),
        AI_BACKEND_NATIVE => resolve_native(cap),
        _ => resolve_effective(cap),
    }
}

fn resolve_trusted_cli(cap: &AiBackendCapabilities) -> BackendSendDecision {
    if cap.trusted_cli_available {
        return BackendSendDecision::Use(AI_BACKEND_TRUSTED_CLI);
    }

    let reason = cap
        .trusted_cli_reason
        .clone()
        .or_else(|| cap.effective_backend_reason.clone())
        .unwrap_or_else(|| "trusted-cli unavailable".to_string());
    if cap.native_available {
        return BackendSendDecision::Switch {
            backend: AI_BACKEND_NATIVE,
            reason,
        };
    }
    BackendSendDecision::Block { reason }
}

fn resolve_native(cap: &AiBackendCapabilities) -> BackendSendDecision {
    if cap.native_available {
        return BackendSendDecision::Use(AI_BACKEND_NATIVE);
    }

    let reason = cap
        .native_reason
        .clone()
        .or_else(|| cap.effective_backend_reason.clone())
        .unwrap_or_else(|| "native AI disabled by config".to_string());
    if cap.effective_backend == AI_BACKEND_TRUSTED_CLI && cap.trusted_cli_available {
        return BackendSendDecision::Switch {
            backend: AI_BACKEND_TRUSTED_CLI,
            reason,
        };
    }
    BackendSendDecision::Block { reason }
}

fn resolve_effective(cap: &AiBackendCapabilities) -> BackendSendDecision {
    match cap.effective_backend.as_str() {
        AI_BACKEND_NATIVE if cap.native_available => BackendSendDecision::Use(AI_BACKEND_NATIVE),
        AI_BACKEND_TRUSTED_CLI if cap.trusted_cli_available => {
            BackendSendDecision::Use(AI_BACKEND_TRUSTED_CLI)
        }
        _ => BackendSendDecision::Block {
            reason: cap
                .effective_backend_reason
                .clone()
                .or_else(|| cap.native_reason.clone())
                .or_else(|| cap.trusted_cli_reason.clone())
                .unwrap_or_else(|| "no AI backend available".to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_product_backend_names_to_runtime_plugin_ids() {
        assert_eq!(ai_backend_to_plugin_id(AI_BACKEND_NATIVE), AI_PLUGIN_NATIVE);
        assert_eq!(
            ai_backend_to_plugin_id(AI_BACKEND_TRUSTED_CLI),
            AI_PLUGIN_TRUSTED_CLI
        );
        assert_eq!(ai_backend_to_plugin_id("unknown"), AI_PLUGIN_NATIVE);
    }

    #[test]
    fn backend_for_send_uses_native_when_native_is_available() {
        assert_eq!(
            resolve_backend_for_send(AI_BACKEND_NATIVE, &AiBackendCapabilities::default()),
            BackendSendDecision::Use(AI_BACKEND_NATIVE)
        );
    }

    #[test]
    fn backend_for_send_falls_back_to_native_when_trusted_cli_is_blocked() {
        let cap = AiBackendCapabilities {
            native_available: true,
            trusted_cli_available: false,
            trusted_cli_reason: Some("trusted mode required".to_string()),
            effective_backend: AI_BACKEND_NATIVE.to_string(),
            effective_backend_reason: Some("trusted mode required".to_string()),
            ..AiBackendCapabilities::default()
        };

        assert_eq!(
            resolve_backend_for_send(AI_BACKEND_TRUSTED_CLI, &cap),
            BackendSendDecision::Switch {
                backend: AI_BACKEND_NATIVE,
                reason: "trusted mode required".to_string()
            }
        );
    }

    #[test]
    fn backend_for_send_blocks_when_no_backend_is_available() {
        let cap = AiBackendCapabilities::unavailable("native AI disabled by config");

        assert_eq!(
            resolve_backend_for_send(AI_BACKEND_NATIVE, &cap),
            BackendSendDecision::Block {
                reason: "native AI disabled by config".to_string()
            }
        );
    }

    #[test]
    fn backend_for_send_switches_to_trusted_cli_when_server_effective_backend_allows_it() {
        let cap = AiBackendCapabilities {
            native_available: false,
            native_reason: Some("native AI disabled by config".to_string()),
            trusted_cli_available: true,
            effective_backend: AI_BACKEND_TRUSTED_CLI.to_string(),
            effective_backend_reason: Some("trusted-cli explicitly requested".to_string()),
            ..AiBackendCapabilities::default()
        };

        assert_eq!(
            resolve_backend_for_send(AI_BACKEND_NATIVE, &cap),
            BackendSendDecision::Switch {
                backend: AI_BACKEND_TRUSTED_CLI,
                reason: "native AI disabled by config".to_string()
            }
        );
    }
}
