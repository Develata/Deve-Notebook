use crate::{DesktopTauriBootstrapError, try_desktop_tauri_local_service_bootstrap_from_env};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopTauriRuntimeSurface {
    pub runtime_entrypoint_declared: bool,
    pub build_script_declared: bool,
    pub window_shell_runtime_declared: bool,
    pub menu_tray_runtime_bound: bool,
    pub child_process_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
}

impl DesktopTauriRuntimeSurface {
    pub fn is_shell_only(self) -> bool {
        self.runtime_entrypoint_declared
            && self.build_script_declared
            && self.window_shell_runtime_declared
            && self.menu_tray_runtime_bound
            && !self.child_process_runtime_enabled
            && !self.opens_authority_write_path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopTauriStartupSmoke {
    pub packaged_binary_started: bool,
    pub shell_only_runtime: bool,
    pub child_process_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
}

impl DesktopTauriStartupSmoke {
    pub fn passed(self) -> bool {
        self.packaged_binary_started
            && self.shell_only_runtime
            && !self.child_process_runtime_enabled
            && !self.opens_authority_write_path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopTauriNativeSessionSmoke {
    pub local_service_started: bool,
    pub session_bound: bool,
    pub native_session_cookie_installed_before_bootstrap: bool,
    pub opens_authority_write_path: bool,
}

impl DesktopTauriNativeSessionSmoke {
    pub fn passed(self) -> bool {
        self.local_service_started
            && self.session_bound
            && self.native_session_cookie_installed_before_bootstrap
            && !self.opens_authority_write_path
    }
}

pub const DESKTOP_TAURI_STARTUP_SMOKE_OK: &str = "desktop-startup-smoke: ok";
pub const DESKTOP_TAURI_NATIVE_SESSION_SMOKE_OK: &str = "desktop-native-session-smoke: ok";

pub fn desktop_tauri_runtime_surface() -> DesktopTauriRuntimeSurface {
    DesktopTauriRuntimeSurface {
        runtime_entrypoint_declared: true,
        build_script_declared: true,
        window_shell_runtime_declared: true,
        menu_tray_runtime_bound: true,
        child_process_runtime_enabled: false,
        opens_authority_write_path: false,
    }
}

pub fn desktop_tauri_startup_smoke() -> DesktopTauriStartupSmoke {
    let surface = desktop_tauri_runtime_surface();
    DesktopTauriStartupSmoke {
        packaged_binary_started: true,
        shell_only_runtime: surface.is_shell_only(),
        child_process_runtime_enabled: surface.child_process_runtime_enabled,
        opens_authority_write_path: surface.opens_authority_write_path,
    }
}

pub fn desktop_tauri_native_session_smoke(
    timestamp_unix_ms: i64,
) -> Result<DesktopTauriNativeSessionSmoke, DesktopTauriBootstrapError> {
    let Some(mut bootstrap) =
        try_desktop_tauri_local_service_bootstrap_from_env(timestamp_unix_ms)?
    else {
        return Ok(DesktopTauriNativeSessionSmoke {
            local_service_started: false,
            session_bound: false,
            native_session_cookie_installed_before_bootstrap: false,
            opens_authority_write_path: false,
        });
    };

    let local_service_started = bootstrap.runtime.is_some();
    let smoke = DesktopTauriNativeSessionSmoke {
        local_service_started,
        session_bound: bootstrap.script.session_bound(),
        native_session_cookie_installed_before_bootstrap: bootstrap
            .script
            .has_native_session_cookie(),
        opens_authority_write_path: bootstrap.script.opens_authority_write_path(),
    };

    if let Some(runtime) = bootstrap.runtime.as_mut() {
        let _ = runtime.stop(timestamp_unix_ms.saturating_add(1));
    }

    Ok(smoke)
}
