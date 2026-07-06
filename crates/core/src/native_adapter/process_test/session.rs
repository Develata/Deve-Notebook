use crate::native_adapter::{
    CURRENT_NATIVE_PROCESS_ADAPTER_POLICY, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV,
    NativeProcessAdapter, NativeProcessAdapterError, NativeProcessAdapterState,
    NativeProcessEnvBinding, NativeProcessRuntimeSnapshot, NativeProcessRuntimeState,
};

use super::endpoint;

#[test]
fn session_handoff_requires_existing_endpoint() {
    let mut adapter = NativeProcessAdapter::default();

    assert_eq!(
        adapter.bind_session(true),
        Err(NativeProcessAdapterError::EndpointNotBound)
    );

    adapter
        .bind_existing_endpoint(endpoint("http://127.0.0.1:3001", "ws://127.0.0.1:3001"))
        .expect("bind endpoint");

    assert_eq!(
        adapter.bind_session(false),
        Err(NativeProcessAdapterError::SessionNotBound)
    );

    let snapshot = adapter.bind_session(true).expect("bind session");
    assert_eq!(
        snapshot.state,
        NativeProcessAdapterState::SessionHandoffReady
    );
    assert!(snapshot.endpoint.expect("endpoint").session_bound);
}

#[test]
fn process_runtime_snapshot_never_serializes_secret_token_or_output_payload() {
    let snapshot =
        NativeProcessRuntimeSnapshot::disabled_by_policy(CURRENT_NATIVE_PROCESS_ADAPTER_POLICY);
    let encoded = serde_json::to_string(&snapshot).expect("serialize snapshot");

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Disabled);
    assert!(!snapshot.child_process_runtime_enabled);
    assert!(!snapshot.authority_writes_allowed);
    assert!(!encoded.contains("secret"));
    assert!(!encoded.contains("token"));
    assert!(!encoded.contains("stdout"));
    assert!(!encoded.contains("stderr"));
}

#[test]
fn process_env_binding_debug_and_serde_redact_secret_values() {
    let binding = NativeProcessEnvBinding {
        key: NATIVE_SESSION_BOOTSTRAP_SECRET_ENV.to_string(),
        value: "native-secret-value".to_string(),
    };

    let debug = format!("{binding:?}");
    let encoded = serde_json::to_string(&binding).expect("serialize env binding");

    assert!(debug.contains("<redacted>"));
    assert!(encoded.contains("<redacted>"));
    assert!(!debug.contains("native-secret-value"));
    assert!(!encoded.contains("native-secret-value"));
}
