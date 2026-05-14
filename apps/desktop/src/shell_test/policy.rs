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
fn desktop_default_build_opens_only_dependency_spike() {
    let policy = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(
        policy.decision,
        NativePackagingDependencyGateDecision::DesktopAndMobileDependencySpikeOpen
    );
    assert!(policy.is_desktop_dependency_spike_open());
    assert!(policy.desktop_tauri_dependencies_allowed);
    assert!(policy.mobile_tauri_dependencies_allowed);
    assert!(policy.default_build_remains_no_tauri);
    assert!(!policy.authority_writes_allowed);
}
