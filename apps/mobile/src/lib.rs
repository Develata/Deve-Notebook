//! plan_ref:
//!   - 11_ui_design_03_mobile#mobile-native-adapter-contract
//!
//! Minimal mobile native shell skeleton.
//!
//! The default build intentionally avoids Tauri. Optional packaging
//! dependencies are isolated behind the `native-packaging` feature. This crate
//! models the shell's allowed responsibilities: bind to a controlled local
//! service, bind a short-lived session, inject a Web bootstrap object, and force
//! foreground reprobe after mobile lifecycle transitions.

#[cfg(feature = "native-packaging")]
mod packaging;
#[cfg(all(test, feature = "native-packaging"))]
mod packaging_test;
mod shell;
#[cfg(test)]
mod shell_recovery_test;
#[cfg(test)]
mod shell_test;
#[cfg(feature = "native-packaging")]
mod tauri_entry;
mod types;

#[cfg(feature = "native-packaging")]
pub use packaging::{
    MOBILE_ANDROID_PACKAGE_GATE_ANCHOR, MOBILE_ANDROID_PACKAGE_SCRIPT,
    MOBILE_IOS_PACKAGE_GATE_ANCHOR, MOBILE_IOS_PACKAGE_SCRIPT, MOBILE_TAURI_CONFIG_PATH,
    MOBILE_TAURI_IDENTIFIER, MOBILE_TAURI_MAIN_WINDOW_LABEL, MOBILE_TAURI_MAIN_WINDOW_TITLE,
    MOBILE_TAURI_PRODUCT_NAME, MobileAndroidShellPackageExecution, MobileIosShellPackageExecution,
    MobilePackagingAcceptance, MobilePackagingAuthority, MobilePackagingCapability,
    MobilePackagingDependencyBatch, MobilePackagingScaffold, MobileShellPackagingAcceptance,
    mobile_packaging_scaffold,
};
pub use shell::MobileShell;
#[cfg(feature = "native-packaging")]
pub use tauri_entry::{
    MobileTauriRuntimeSurface, mobile_tauri_runtime_surface, run_mobile_tauri_app,
};
pub use types::{
    MobileBootstrap, MobileLifecycleEvent, MobileRecoveryBootstrap, MobileServiceState,
    MobileSessionMaterial, MobileShellError, MobileShellSnapshot,
};
