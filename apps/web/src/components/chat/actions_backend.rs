//! plan_ref:
//!   - 10_ai_agent#trusted-agent-bridge
//!
//! Send-time backend policy for AI chat actions.

use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum BackendSendDecision {
    Use(&'static str),
    Switch {
        backend: &'static str,
        reason: String,
    },
    Block {
        reason: String,
    },
}

pub(super) fn resolve_backend_for_send(
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
#[path = "actions_backend_test.rs"]
mod tests;
