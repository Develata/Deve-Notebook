//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!
//! Minimal mobile native shell skeleton.
//!
//! This crate intentionally avoids depending on Tauri Mobile for the first
//! boundary pass. It models the shell's allowed responsibilities: bind to a
//! controlled local service, bind a short-lived session, inject a Web bootstrap
//! object, and force foreground reprobe after mobile lifecycle transitions.

#[cfg(feature = "native-packaging")]
mod packaging;
#[cfg(all(test, feature = "native-packaging"))]
mod packaging_test;
mod shell;
#[cfg(test)]
mod shell_recovery_test;
#[cfg(test)]
mod shell_test;
mod types;

#[cfg(feature = "native-packaging")]
pub use packaging::{
    MobilePackagingAcceptance, MobilePackagingAuthority, MobilePackagingCapability,
    MobilePackagingDependencyBatch, MobilePackagingScaffold, mobile_packaging_scaffold,
};
pub use shell::MobileShell;
pub use types::{
    MobileBootstrap, MobileLifecycleEvent, MobileRecoveryBootstrap, MobileServiceState,
    MobileSessionMaterial, MobileShellError, MobileShellSnapshot,
};
