use deve_core::native_adapter::{
    CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY, CURRENT_NATIVE_PROCESS_ADAPTER_POLICY,
    NativePackagingDependencyGateDecision, NativeProcessAdapterDecision,
};

#[test]
fn desktop_default_build_defers_real_process_adapter() {
    let policy = CURRENT_NATIVE_PROCESS_ADAPTER_POLICY;

    assert_eq!(
        policy.decision,
        NativeProcessAdapterDecision::DeferredUntilPackagingGate
    );
    assert!(policy.is_deferred_no_runtime());
    assert!(!policy.child_process_runtime_enabled);
    assert!(!policy.authority_writes_allowed);
}

#[test]
fn desktop_default_build_keeps_packaging_dependency_gate_closed() {
    let policy = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(
        policy.decision,
        NativePackagingDependencyGateDecision::DeferredUntilRuntimeBatch
    );
    assert!(policy.is_deferred_no_dependency());
    assert!(!policy.real_tauri_dependencies_allowed);
    assert!(policy.default_build_remains_no_tauri);
    assert!(!policy.authority_writes_allowed);
}
