//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
//! Platform-neutral native adapter contract.
//!
//! The native shell is allowed to bind the web application to a controlled
//! local service. It is not allowed to become authority for ledger, Projection Locator,
//! source-control, search, or repo-scope write decisions.

#[cfg(not(target_arch = "wasm32"))]
mod loopback_http;
mod packaging;
mod process;
mod process_runtime;
mod shell_core;
mod supervisor;
mod types;
mod validation;

#[cfg(not(target_arch = "wasm32"))]
pub use loopback_http::{
    DEFAULT_LOOPBACK_HTTP_RETRY_INTERVAL, DEFAULT_LOOPBACK_HTTP_STARTUP_GRACE,
    DEFAULT_LOOPBACK_HTTP_TIMEOUT, DEFAULT_MAX_LOOPBACK_RESPONSE_BYTES, NativeLoopbackHttpError,
    NativeLoopbackHttpProbe, NativeLoopbackHttpResponse, NativeLoopbackHttpTarget,
    is_retryable_startup_probe_error, loopback_host_from_http_base, parse_loopback_http_url,
};
pub use packaging::{
    CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY, NativePackagingDependencyGateDecision,
    NativePackagingDependencyGatePolicy,
};
pub use process::{
    CURRENT_NATIVE_PROCESS_ADAPTER_POLICY, DEVE_DESKTOP_LOCAL_SERVICE_ENV,
    DEVE_MOBILE_EMBEDDED_SERVICE_ENV, DEVE_NATIVE_AUTHORITY_ENV, NativeProcessAdapter,
    NativeProcessAdapterDecision, NativeProcessAdapterError, NativeProcessAdapterPolicy,
    NativeProcessAdapterSnapshot, NativeProcessAdapterState, NativeProcessEnvPolicyError,
    NativeRuntimeEnvConfig, NativeRuntimeEnvPolicy, desktop_local_backend_policy,
    desktop_native_authority_policy_from_env, mobile_local_backend_policy,
    mobile_native_authority_policy_from_env, parse_optional_env_flag, parse_optional_flag_value,
};
pub use process_runtime::{
    NativeProcessBindHints, NativeProcessEnvBinding, NativeProcessExitStatus,
    NativeProcessPathResolution, NativeProcessRuntimeError, NativeProcessRuntimeEvent,
    NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle, NativeProcessRuntimeSnapshot,
    NativeProcessRuntimeState, NativeProcessSpawnSpec,
};
pub use shell_core::{NativeShellCore, NativeShellCoreSnapshot};
pub use supervisor::{
    NativeServiceFailureKind, NativeServiceHealthProbe, NativeServiceSupervisor,
    NativeServiceSupervisorError, NativeServiceSupervisorObservation,
    NativeServiceSupervisorSnapshot, NativeServiceSupervisorState,
};
pub use types::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV,
    NATIVE_TAURI_CUSTOM_PROTOCOL_ORIGIN, NATIVE_TAURI_HTTP_LOCALHOST_ORIGIN, NativeAdapterPlatform,
    NativeAdapterSnapshot, NativeAdapterState, NativeEndpointReady, NativePlatformEvent,
    NativePlatformEventEffect, NativePlatformEventKind, NativeRemoteTarget, NativeRuntimeReadiness,
    NativeServiceOffline, NativeServiceRestarting, NativeServiceSuspended, NativeShellMode,
    classify_native_platform_event, native_tauri_allowed_origins, platform_event_can_grant_write,
};
pub use validation::{
    NativeAdapterError, can_load_native_web_shell, can_show_native_writable_shell,
    validate_native_endpoint_bases, validate_native_endpoint_ready, validate_native_remote_target,
};

#[cfg(test)]
mod contract_test;
#[cfg(test)]
mod process_test;
#[cfg(test)]
mod supervisor_test;
