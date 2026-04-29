//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!
//! Platform-neutral native adapter contract.
//!
//! The native shell is allowed to bind the web application to a controlled
//! local service. It is not allowed to become authority for ledger, vault,
//! source-control, search, or repo-scope write decisions.

mod process;
mod supervisor;
mod types;
mod validation;

pub use process::{
    CURRENT_NATIVE_PROCESS_ADAPTER_POLICY, NativeProcessAdapterDecision, NativeProcessAdapterPolicy,
};
pub use supervisor::{
    NativeServiceFailureKind, NativeServiceHealthProbe, NativeServiceSupervisor,
    NativeServiceSupervisorError, NativeServiceSupervisorSnapshot, NativeServiceSupervisorState,
};
pub use types::{
    NativeAdapterPlatform, NativeAdapterSnapshot, NativeAdapterState, NativeEndpointReady,
    NativePlatformEvent, NativePlatformEventEffect, NativePlatformEventKind,
    NativeRuntimeReadiness, NativeServiceOffline, NativeServiceRestarting, NativeServiceSuspended,
    classify_native_platform_event, platform_event_can_grant_write,
};
pub use validation::{
    NativeAdapterError, can_load_native_web_shell, can_show_native_writable_shell,
    validate_native_endpoint_bases, validate_native_endpoint_ready,
};

#[cfg(test)]
mod contract_test;
