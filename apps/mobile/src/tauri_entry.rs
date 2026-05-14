//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_03_mobile#mobile-android-shell-package-execution-gate
//!
//! Android shell-only Tauri entrypoint.
//!
//! This module starts only the mobile WebView shell. It does not spawn the
//! Deve backend service and does not write ledger, vault, source-control,
//! search, Git, or `.notegit` authority.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileTauriRuntimeSurface {
    pub android_shell_package_entrypoint_declared: bool,
    pub build_script_declared: bool,
    pub webview_shell_runtime_declared: bool,
    pub child_process_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
    pub release_ready_claimed: bool,
}

impl MobileTauriRuntimeSurface {
    pub fn is_shell_only(self) -> bool {
        self.android_shell_package_entrypoint_declared
            && self.build_script_declared
            && self.webview_shell_runtime_declared
            && !self.child_process_runtime_enabled
            && !self.opens_authority_write_path
            && !self.release_ready_claimed
    }
}

pub fn mobile_tauri_runtime_surface() -> MobileTauriRuntimeSurface {
    MobileTauriRuntimeSurface {
        android_shell_package_entrypoint_declared: true,
        build_script_declared: true,
        webview_shell_runtime_declared: true,
        child_process_runtime_enabled: false,
        opens_authority_write_path: false,
        release_ready_claimed: false,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_mobile_tauri_app() {
    let result = tauri::Builder::default().run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("deve_mobile Tauri shell exited with error: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobile_tauri_runtime_surface_is_shell_only() {
        let surface = mobile_tauri_runtime_surface();

        assert!(surface.is_shell_only());
        assert!(surface.android_shell_package_entrypoint_declared);
        assert!(surface.build_script_declared);
        assert!(surface.webview_shell_runtime_declared);
        assert!(!surface.child_process_runtime_enabled);
        assert!(!surface.opens_authority_write_path);
        assert!(!surface.release_ready_claimed);
    }
}
