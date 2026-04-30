use super::*;

fn endpoint(http_base: &str, ws_base: &str) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: http_base.to_string(),
        ws_base: ws_base.to_string(),
        node_role: "native-main".to_string(),
        session_bound: true,
    }
}

#[test]
fn current_policy_defers_real_process_runtime() {
    let policy = CURRENT_NATIVE_PROCESS_ADAPTER_POLICY;

    assert!(policy.is_deferred_no_runtime());
    assert!(!policy.child_process_runtime_enabled);
    assert!(policy.packaging_gate_required);
    assert!(!policy.authority_writes_allowed);
}

#[test]
fn default_adapter_rejects_child_process_runtime() {
    let adapter = NativeProcessAdapter::default();
    let snapshot = adapter.snapshot();

    assert!(adapter.ensure_child_process_runtime_enabled().is_err());
    assert!(snapshot.is_default_safe_boundary());
    assert_eq!(snapshot.state, NativeProcessAdapterState::Deferred);
}

#[test]
fn default_adapter_binds_existing_loopback_service_without_runtime() {
    let mut adapter = NativeProcessAdapter::default();

    let snapshot = adapter
        .bind_existing_endpoint(endpoint("http://127.0.0.1:3001", "ws://localhost:3001"))
        .expect("bind endpoint");

    assert_eq!(
        snapshot.state,
        NativeProcessAdapterState::ExistingEndpointBound
    );
    assert!(snapshot.health_probe.is_healthy());
    assert!(!snapshot.endpoint.as_ref().expect("endpoint").session_bound);
    assert!(snapshot.is_default_safe_boundary());
}

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
fn default_adapter_rejects_non_loopback_existing_endpoint() {
    let mut adapter = NativeProcessAdapter::default();

    assert!(matches!(
        adapter.bind_existing_endpoint(endpoint("http://192.168.1.10:3001", "ws://127.0.0.1:3001")),
        Err(NativeProcessAdapterError::InvalidEndpoint(
            NativeAdapterError::NonLoopbackHost { field: "http_base" }
        ))
    ));
}
