//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!
//! Minimal desktop native shell skeleton.
//!
//! The default build intentionally avoids Tauri; optional native packaging
//! dependencies are isolated behind the `native-packaging` feature. This crate
//! models the shell's allowed responsibilities: bind to a controlled local
//! service, bind a short-lived session out of band, inject a Web bootstrap
//! object, and report service recovery state.

#[cfg(feature = "native-packaging")]
mod menu_tray;
#[cfg(feature = "native-packaging")]
mod packaging;
#[cfg(all(test, feature = "native-packaging"))]
mod packaging_test;
#[cfg(feature = "native-packaging")]
mod process_runtime;
#[cfg(all(test, feature = "native-packaging"))]
mod process_runtime_test;
#[cfg(feature = "native-packaging")]
mod service_bootstrap;
#[cfg(all(test, feature = "native-packaging"))]
mod service_bootstrap_test;
#[cfg(feature = "native-packaging")]
mod service_entrypoint;
#[cfg(all(test, feature = "native-packaging"))]
mod service_entrypoint_test;
mod shell;
#[cfg(test)]
mod shell_recovery_test;
#[cfg(test)]
mod shell_test;
#[cfg(feature = "native-packaging")]
mod tauri_bootstrap;
#[cfg(all(test, feature = "native-packaging"))]
mod tauri_bootstrap_test;
#[cfg(feature = "native-packaging")]
mod tauri_entry;
mod types;

#[cfg(feature = "native-packaging")]
pub use menu_tray::{
    DESKTOP_MENU_APP_OPEN_COMMAND_PALETTE_ID, DESKTOP_MENU_APP_OPEN_SETTINGS_ID,
    DESKTOP_MENU_APP_QUIT_REQUESTED_ID, DESKTOP_MENU_APP_SHOW_MAIN_WINDOW_ID,
    DESKTOP_MENU_HELP_OPEN_COMMAND_PALETTE_ID, DESKTOP_MENU_ROOT_ID,
    DESKTOP_MENU_WINDOW_SHOW_MAIN_WINDOW_ID, DESKTOP_TRAY_MENU_ID, DESKTOP_TRAY_QUIT_REQUESTED_ID,
    DESKTOP_TRAY_SHOW_MAIN_WINDOW_ID, DESKTOP_TRAY_TOGGLE_WINDOW_VISIBILITY_ID, build_desktop_menu,
    build_desktop_tray_icon, build_desktop_tray_menu, resolve_desktop_menu_action_id,
    resolve_desktop_tray_action_id,
};
#[cfg(feature = "native-packaging")]
pub use packaging::{
    DESKTOP_MENU_APP_ID, DESKTOP_MENU_HELP_ID, DESKTOP_MENU_WINDOW_ID, DESKTOP_TAURI_CONFIG_PATH,
    DESKTOP_TAURI_IDENTIFIER, DESKTOP_TAURI_MAIN_WINDOW_LABEL, DESKTOP_TAURI_MAIN_WINDOW_TITLE,
    DESKTOP_TAURI_PRODUCT_NAME, DESKTOP_TRAY_ID, DesktopMenuAction, DesktopMenuTraySurface,
    DesktopPackagingAcceptance, DesktopPackagingAuthority, DesktopPackagingCapability,
    DesktopPackagingDependencyBatch, DesktopPackagingScaffold, DesktopShellPackagingAcceptance,
    DesktopTrayAction, desktop_packaging_scaffold,
};
#[cfg(feature = "native-packaging")]
pub use process_runtime::{
    DesktopCommandProcessLauncher, DesktopLocalServiceRuntime, DesktopProcessLauncher,
    DesktopProcessRuntimeError,
};
#[cfg(feature = "native-packaging")]
pub use service_bootstrap::{
    DesktopLocalServiceBootstrapError, DesktopLocalServiceBootstrapResult,
    DesktopLocalServiceProbe, DesktopLocalServiceProbeOutcome, DesktopLocalServiceSessionHandoff,
    DesktopLoopbackHttpProbe, node_role_probe_outcome_from_json,
    run_desktop_local_service_bootstrap, session_material_from_auth_status_json,
};
#[cfg(feature = "native-packaging")]
pub use service_entrypoint::{
    DEVE_DESKTOP_LOCAL_SERVICE_ENV, DEVE_NATIVE_AUTHORITY_ENV, DesktopLocalServiceEntrypointError,
    DesktopLocalServiceEntrypointInput, DesktopLocalServiceEntrypointPlan,
    DesktopLocalServiceEntrypointPolicy, desktop_local_service_entrypoint_policy_from_env,
    plan_desktop_local_service_entrypoint, plan_desktop_local_service_entrypoint_from_env,
};
pub use shell::DesktopShell;
#[cfg(feature = "native-packaging")]
pub use tauri_bootstrap::{
    DesktopLocalServiceTauriState, DesktopTauriBootstrapError, DesktopTauriBootstrapScript,
    DesktopTauriLocalServiceBootstrap, desktop_tauri_bootstrap_plugin,
    desktop_tauri_local_service_bootstrap_from_env, desktop_tauri_recovery_init_script,
    desktop_tauri_service_offline_init_script, desktop_tauri_session_invalid_init_script,
    desktop_tauri_success_init_script, try_desktop_tauri_local_service_bootstrap_from_env,
};
#[cfg(feature = "native-packaging")]
pub use tauri_entry::{
    DESKTOP_TAURI_NATIVE_SESSION_SMOKE_OK, DESKTOP_TAURI_STARTUP_SMOKE_OK,
    DesktopTauriNativeSessionSmoke, DesktopTauriRuntimeSurface, DesktopTauriShellEffect,
    DesktopTauriStartupSmoke, desktop_tauri_native_session_smoke, desktop_tauri_runtime_surface,
    desktop_tauri_startup_smoke, menu_action_shell_effect, run_desktop_tauri_app,
    tray_action_shell_effect,
};
pub use types::{
    DesktopBootstrap, DesktopNativeSessionCookie, DesktopRecoveryBootstrap, DesktopServiceState,
    DesktopSessionMaterial, DesktopShellError, DesktopShellSnapshot,
};
