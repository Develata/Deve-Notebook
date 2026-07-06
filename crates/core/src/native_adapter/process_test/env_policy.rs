use crate::native_adapter::{
    DEVE_DESKTOP_LOCAL_SERVICE_ENV, DEVE_MOBILE_EMBEDDED_SERVICE_ENV, DEVE_NATIVE_AUTHORITY_ENV,
    NativeRuntimeEnvConfig, NativeRuntimeEnvPolicy, desktop_native_authority_policy_from_env,
    mobile_native_authority_policy_from_env,
};

use super::{ENV_LOCK, EnvGuard};

#[test]
fn desktop_native_authority_opt_in_requires_both_env_flags() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(&[
        (DEVE_NATIVE_AUTHORITY_ENV, Some("1")),
        (DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("1")),
        (DEVE_MOBILE_EMBEDDED_SERVICE_ENV, None),
    ]);

    let policy = desktop_native_authority_policy_from_env();

    assert!(policy.is_explicit_desktop_native_authority_opt_in());
    assert!(policy.child_process_runtime_enabled);
    assert!(!policy.embedded_service_runtime_enabled);
    assert!(policy.authority_writes_allowed);
}

#[test]
fn mobile_native_authority_opt_in_uses_embedded_service_without_child_process() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(&[
        (DEVE_NATIVE_AUTHORITY_ENV, Some("1")),
        (DEVE_DESKTOP_LOCAL_SERVICE_ENV, None),
        (DEVE_MOBILE_EMBEDDED_SERVICE_ENV, Some("1")),
    ]);

    let policy = mobile_native_authority_policy_from_env();

    assert!(policy.is_explicit_mobile_native_authority_opt_in());
    assert!(!policy.child_process_runtime_enabled);
    assert!(policy.embedded_service_runtime_enabled);
    assert!(policy.authority_writes_allowed);
}

#[test]
fn desktop_native_authority_policy_ignores_invalid_mobile_env() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(&[
        (DEVE_NATIVE_AUTHORITY_ENV, Some("1")),
        (DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("1")),
        (DEVE_MOBILE_EMBEDDED_SERVICE_ENV, Some("invalid")),
    ]);

    let policy = desktop_native_authority_policy_from_env();

    assert!(policy.is_explicit_desktop_native_authority_opt_in());
    assert!(policy.child_process_runtime_enabled);
    assert!(!policy.embedded_service_runtime_enabled);
    assert!(policy.authority_writes_allowed);
}

#[test]
fn mobile_native_authority_policy_ignores_invalid_desktop_env() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(&[
        (DEVE_NATIVE_AUTHORITY_ENV, Some("1")),
        (DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("invalid")),
        (DEVE_MOBILE_EMBEDDED_SERVICE_ENV, Some("1")),
    ]);

    let policy = mobile_native_authority_policy_from_env();

    assert!(policy.is_explicit_mobile_native_authority_opt_in());
    assert!(!policy.child_process_runtime_enabled);
    assert!(policy.embedded_service_runtime_enabled);
    assert!(policy.authority_writes_allowed);
}

#[test]
fn runtime_env_policy_keeps_local_backend_enable_separate_from_authority_flag() {
    let policy = NativeRuntimeEnvPolicy::from_config(NativeRuntimeEnvConfig {
        native_authority: Some(false),
        desktop_local_service: None,
        mobile_embedded_service: None,
    });

    assert!(policy.desktop_local_backend_enabled);
    assert!(policy.mobile_embedded_backend_enabled);
    assert!(policy.desktop_authority_policy.is_deferred_no_runtime());
    assert!(policy.mobile_authority_policy.is_deferred_no_runtime());

    let disabled = NativeRuntimeEnvPolicy::from_config(NativeRuntimeEnvConfig {
        native_authority: Some(true),
        desktop_local_service: Some(false),
        mobile_embedded_service: Some(false),
    });

    assert!(!disabled.desktop_local_backend_enabled);
    assert!(!disabled.mobile_embedded_backend_enabled);
    assert!(disabled.desktop_authority_policy.is_deferred_no_runtime());
    assert!(disabled.mobile_authority_policy.is_deferred_no_runtime());
}
