//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!
//! Minimal desktop native shell skeleton.
//!
//! This crate intentionally avoids depending on Tauri for the first boundary
//! pass. It models the shell's allowed responsibilities: bind to a controlled
//! local service, bind a short-lived session out of band, inject a Web bootstrap
//! object, and report service recovery state.

mod shell;
#[cfg(test)]
mod shell_test;

pub use shell::{
    DesktopBootstrap, DesktopRecoveryBootstrap, DesktopServiceState, DesktopSessionMaterial,
    DesktopShell, DesktopShellError, DesktopShellSnapshot,
};
