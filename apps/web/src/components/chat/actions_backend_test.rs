use super::{BackendSendDecision, resolve_backend_for_send};
use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};

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
fn backend_for_send_switches_to_trusted_cli_only_when_server_effective_backend_allows_it() {
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
