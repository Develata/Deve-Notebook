//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!
//! Minimal mobile native shell skeleton.
//!
//! This crate intentionally avoids depending on Tauri Mobile for the first
//! boundary pass. It models the shell's allowed responsibilities: bind to a
//! controlled local service, bind a short-lived session, inject a Web bootstrap
//! object, and force foreground reprobe after mobile lifecycle transitions.

mod shell;
#[cfg(test)]
mod shell_test;
mod types;

pub use shell::MobileShell;
pub use types::{
    MobileBootstrap, MobileLifecycleEvent, MobileRecoveryBootstrap, MobileServiceState,
    MobileSessionMaterial, MobileShellError, MobileShellSnapshot,
};
