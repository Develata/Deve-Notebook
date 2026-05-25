//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!
//! Platform-neutral native adapter contract.
//!
//! The native shell is allowed to bind the web application to a controlled
//! local service. It is not allowed to become authority for ledger, Projection Locator,
//! source-control, search, or repo-scope write decisions.

mod packaging;
mod process;
mod process_runtime;
mod shell_core;
mod supervisor;
mod types;
mod validation;

pub use packaging::{
    CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY, NativePackagingDependencyGateDecision,
    NativePackagingDependencyGatePolicy,
};
pub use process::{
    CURRENT_NATIVE_PROCESS_ADAPTER_POLICY, NativeProcessAdapter, NativeProcessAdapterDecision,
    NativeProcessAdapterError, NativeProcessAdapterPolicy, NativeProcessAdapterSnapshot,
    NativeProcessAdapterState,
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
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, NativeAdapterPlatform,
    NativeAdapterSnapshot, NativeAdapterState, NativeEndpointReady, NativePlatformEvent,
    NativePlatformEventEffect, NativePlatformEventKind, NativeRuntimeReadiness,
    NativeServiceOffline, NativeServiceRestarting, NativeServiceSuspended,
    classify_native_platform_event, platform_event_can_grant_write,
};
pub use validation::{
    NativeAdapterError, can_load_native_web_shell, can_show_native_writable_shell,
    validate_native_endpoint_bases, validate_native_endpoint_ready,
};

#[cfg(test)]
mod contract_test;
#[cfg(test)]
mod process_test;
#[cfg(test)]
mod supervisor_test;
