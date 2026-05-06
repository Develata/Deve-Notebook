//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!
//! Minimal desktop native shell skeleton.
//!
//! This crate intentionally avoids depending on Tauri for the first boundary
//! pass. It models the shell's allowed responsibilities: bind to a controlled
//! local service, bind a short-lived session out of band, inject a Web bootstrap
//! object, and report service recovery state.

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
pub use packaging::{
    DesktopPackagingAcceptance, DesktopPackagingAuthority, DesktopPackagingCapability,
    DesktopPackagingDependencyBatch, DesktopPackagingScaffold, desktop_packaging_scaffold,
};
pub use shell::{
    DesktopBootstrap, DesktopRecoveryBootstrap, DesktopServiceState, DesktopSessionMaterial,
    DesktopShell, DesktopShellError, DesktopShellSnapshot,
};
