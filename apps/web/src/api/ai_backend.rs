//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
use gloo_net::http::Request;
use serde::Deserialize;
use web_sys::RequestCredentials;

use super::native_http::api_url;

pub const AI_BACKEND_NATIVE: &str = "native";
pub const AI_BACKEND_TRUSTED_CLI: &str = "trusted-cli";
pub const AI_PLUGIN_NATIVE: &str = "ai-chat";
pub const AI_PLUGIN_TRUSTED_CLI: &str = "agent-bridge";

#[derive(Clone, Debug, Deserialize)]
pub struct AiBackendCapabilities {
    pub native_available: bool,
    pub native_reason: Option<String>,
    pub trusted_cli_available: bool,
    pub trusted_cli_reason: Option<String>,
    pub effective_backend: String,
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

pub async fn fetch_ai_backend_capabilities() -> AiBackendCapabilities {
    let api = api_url("/api/ai/backend-capabilities");
    let mut request = Request::get(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request.send().await;
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

pub fn resolve_backend_for_effective_state(
    current_backend: &str,
    cap: &AiBackendCapabilities,
) -> BackendSendDecision {
    match resolve_backend_for_send(current_backend, cap) {
        BackendSendDecision::Use(backend) if backend == current_backend => {
            resolve_effective_override(current_backend, cap)
        }
        BackendSendDecision::Use(backend) => BackendSendDecision::Switch {
            backend,
            reason: effective_backend_reason(backend, cap),
        },
        other => other,
    }
}

fn resolve_effective_override(
    current_backend: &str,
    cap: &AiBackendCapabilities,
) -> BackendSendDecision {
    if current_backend == cap.effective_backend {
        return BackendSendDecision::Use(current_backend_static(current_backend));
    }

    match cap.effective_backend.as_str() {
        AI_BACKEND_NATIVE if cap.native_available => BackendSendDecision::Switch {
            backend: AI_BACKEND_NATIVE,
            reason: effective_backend_reason(AI_BACKEND_NATIVE, cap),
        },
        AI_BACKEND_TRUSTED_CLI if cap.trusted_cli_available => BackendSendDecision::Switch {
            backend: AI_BACKEND_TRUSTED_CLI,
            reason: effective_backend_reason(AI_BACKEND_TRUSTED_CLI, cap),
        },
        _ => BackendSendDecision::Use(current_backend_static(current_backend)),
    }
}

fn current_backend_static(current_backend: &str) -> &'static str {
    match current_backend {
        AI_BACKEND_TRUSTED_CLI => AI_BACKEND_TRUSTED_CLI,
        _ => AI_BACKEND_NATIVE,
    }
}

fn effective_backend_reason(backend: &'static str, cap: &AiBackendCapabilities) -> String {
    cap.effective_backend_reason
        .clone()
        .unwrap_or_else(|| format!("effective backend is {backend}"))
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
mod tests;
