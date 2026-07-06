use crate::native_adapter::{
    CURRENT_NATIVE_PROCESS_ADAPTER_POLICY, NativeAdapterError, NativeProcessAdapter,
    NativeProcessAdapterError, NativeProcessAdapterState, NativeProcessRuntimeFailureKind,
    desktop_local_backend_policy, mobile_local_backend_policy,
};

use super::endpoint;

#[test]
fn current_policy_defers_real_process_runtime() {
    let policy = CURRENT_NATIVE_PROCESS_ADAPTER_POLICY;

    assert!(policy.is_deferred_no_runtime());
    assert!(!policy.child_process_runtime_enabled);
    assert!(!policy.embedded_service_runtime_enabled);
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
fn local_backend_default_policies_enable_runtime_without_shell_authority_writes() {
    let desktop = desktop_local_backend_policy();
    let mobile = mobile_local_backend_policy();

    assert!(desktop.is_desktop_local_backend_default());
    assert!(desktop.child_process_runtime_enabled);
    assert!(!desktop.embedded_service_runtime_enabled);
    assert!(!desktop.authority_writes_allowed);

    assert!(mobile.is_mobile_local_backend_default());
    assert!(!mobile.child_process_runtime_enabled);
    assert!(mobile.embedded_service_runtime_enabled);
    assert!(!mobile.authority_writes_allowed);
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
fn default_adapter_rejects_non_loopback_existing_endpoint() {
    let mut adapter = NativeProcessAdapter::default();

    assert!(matches!(
        adapter.bind_existing_endpoint(endpoint("http://192.168.1.10:3001", "ws://127.0.0.1:3001")),
        Err(NativeProcessAdapterError::InvalidEndpoint(
            NativeAdapterError::NonLoopbackHost { field: "http_base" }
        ))
    ));
}

#[test]
fn process_runtime_failure_contract_marks_only_budgeted_failures_retryable() {
    assert!(NativeProcessRuntimeFailureKind::BindFailed.retryable_by_default());
    assert!(NativeProcessRuntimeFailureKind::HealthProbeFailed.retryable_by_default());
    assert!(NativeProcessRuntimeFailureKind::ProcessExited.retryable_by_default());
    assert!(!NativeProcessRuntimeFailureKind::SessionHandoffFailed.retryable_by_default());
    assert!(!NativeProcessRuntimeFailureKind::SpawnExecutableMissing.retryable_by_default());
    assert!(!NativeProcessRuntimeFailureKind::EnvironmentPolicyViolation.retryable_by_default());
}
