use deve_core::native_adapter::{
    CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY, CURRENT_NATIVE_PROCESS_ADAPTER_POLICY,
    NativePackagingDependencyGateDecision, NativeProcessAdapterDecision,
    mobile_native_authority_policy_from_env,
};

#[test]
fn mobile_default_build_defers_real_process_adapter() {
    let policy = CURRENT_NATIVE_PROCESS_ADAPTER_POLICY;

    assert_eq!(
        policy.decision,
        NativeProcessAdapterDecision::DeferredUntilPackagingGate
    );
    assert!(policy.is_deferred_no_runtime());
    assert!(!policy.child_process_runtime_enabled);
    assert!(!policy.embedded_service_runtime_enabled);
    assert!(!policy.authority_writes_allowed);
}

#[test]
fn mobile_embedded_service_native_authority_opt_in_has_no_child_process() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(&[
        ("DEVE_NATIVE_AUTHORITY", Some("1")),
        ("DEVE_DESKTOP_LOCAL_SERVICE", None),
        ("DEVE_MOBILE_EMBEDDED_SERVICE", Some("1")),
    ]);

    let policy = mobile_native_authority_policy_from_env();

    assert!(policy.is_explicit_mobile_native_authority_opt_in());
    assert!(!policy.child_process_runtime_enabled);
    assert!(policy.embedded_service_runtime_enabled);
}

#[test]
fn mobile_default_build_keeps_mobile_packaging_feature_gated() {
    let policy = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(
        policy.decision,
        NativePackagingDependencyGateDecision::DesktopAndMobileDependencySpikeOpen
    );
    assert!(policy.is_mobile_dependency_spike_open());
    assert!(policy.desktop_tauri_dependencies_allowed);
    assert!(policy.mobile_tauri_dependencies_allowed);
    assert!(policy.default_build_remains_no_tauri);
    assert!(!policy.authority_writes_allowed);
}

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard {
    old: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(vars: &[(&'static str, Option<&str>)]) -> Self {
        let old = vars
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in vars {
            // SAFETY: tests serialize env mutation through ENV_LOCK and restore every key.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
        Self { old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.old.drain(..) {
            // SAFETY: EnvGuard owns restoration for keys it changed while ENV_LOCK is held.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
