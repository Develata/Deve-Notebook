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
fn trusted_cli_default_off_capabilities_default_to_native_backend() {
    let cap = AiBackendCapabilities::default();

    assert!(cap.native_available);
    assert!(!cap.trusted_cli_available);
    assert_eq!(
        cap.trusted_cli_reason.as_deref(),
        Some("external agent disabled")
    );
    assert_eq!(cap.effective_backend, AI_BACKEND_NATIVE);
    assert!(cap.effective_backend_reason.is_none());
}

#[test]
fn capabilities_json_missing_required_fields_is_invalid() {
    let parsed = serde_json::from_value::<AiBackendCapabilities>(serde_json::json!({}));

    assert!(parsed.is_err());
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

#[test]
fn effective_state_keeps_current_backend_when_it_matches_effective_policy() {
    assert_eq!(
        resolve_backend_for_effective_state(AI_BACKEND_NATIVE, &AiBackendCapabilities::default()),
        BackendSendDecision::Use(AI_BACKEND_NATIVE)
    );
}

#[test]
fn effective_state_switches_to_server_effective_backend() {
    let cap = AiBackendCapabilities {
        native_available: true,
        trusted_cli_available: true,
        effective_backend: AI_BACKEND_TRUSTED_CLI.to_string(),
        effective_backend_reason: Some("trusted-cli explicitly requested".to_string()),
        ..AiBackendCapabilities::default()
    };

    assert_eq!(
        resolve_backend_for_effective_state(AI_BACKEND_NATIVE, &cap),
        BackendSendDecision::Switch {
            backend: AI_BACKEND_TRUSTED_CLI,
            reason: "trusted-cli explicitly requested".to_string()
        }
    );
}

#[test]
fn effective_state_blocks_when_no_backend_is_available() {
    let cap = AiBackendCapabilities::unavailable("AI backend capability probe failed");

    assert_eq!(
        resolve_backend_for_effective_state(AI_BACKEND_NATIVE, &cap),
        BackendSendDecision::Block {
            reason: "AI backend capability probe failed".to_string()
        }
    );
}
