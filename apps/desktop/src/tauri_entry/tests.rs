use super::*;
use crate::DEVE_DESKTOP_LOCAL_SERVICE_ENV;

#[test]
fn desktop_tauri_runtime_surface_is_shell_only() {
    assert!(desktop_tauri_runtime_surface().is_shell_only());
    assert!(desktop_tauri_runtime_surface().local_backend_default_enabled);
    assert!(desktop_tauri_runtime_surface().child_process_runtime_enabled);
    assert!(!desktop_tauri_runtime_surface().opens_authority_write_path);
}

#[test]
fn desktop_tauri_startup_smoke_keeps_authority_closed() {
    let smoke = desktop_tauri_startup_smoke();

    assert!(smoke.passed());
    assert!(smoke.packaged_binary_started);
    assert!(smoke.shell_only_runtime);
    assert!(smoke.local_backend_default_enabled);
    assert!(smoke.child_process_runtime_enabled);
    assert!(!smoke.opens_authority_write_path);
}

#[test]
fn desktop_tauri_native_session_smoke_reports_disabled_when_local_backend_disabled() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(DEVE_DESKTOP_LOCAL_SERVICE_ENV, Some("0"));
    let smoke = desktop_tauri_native_session_smoke(1).expect("smoke");

    assert!(!smoke.passed());
    assert!(!smoke.local_service_started);
    assert!(!smoke.session_bound);
    assert!(!smoke.native_session_cookie_installed_before_bootstrap);
    assert!(!smoke.local_service_stopped_after_smoke);
    assert!(!smoke.opens_authority_write_path);
}

#[test]
fn desktop_tauri_native_session_smoke_requires_cleanup_before_passing() {
    let smoke = DesktopTauriNativeSessionSmoke {
        local_service_started: true,
        session_bound: true,
        native_session_cookie_installed_before_bootstrap: true,
        local_service_stopped_after_smoke: false,
        opens_authority_write_path: false,
    };

    assert!(!smoke.passed());
}

#[test]
fn desktop_launch_options_parse_remote_browser_url() {
    let options = DesktopTauriLaunchOptions::from_args(["--remote-url", "https://deve.example"])
        .expect("options");

    assert_eq!(options.remote_url.as_deref(), Some("https://deve.example"));
    assert_eq!(options.local_backend, None);
}

#[test]
fn desktop_launch_options_parse_remote_browser_url_equals_form() {
    let options = DesktopTauriLaunchOptions::from_args(["--remote-url=https://deve.example"])
        .expect("options");

    assert_eq!(options.remote_url.as_deref(), Some("https://deve.example"));
    assert_eq!(options.local_backend, None);
}

#[test]
fn desktop_launch_options_reject_conflicting_local_and_remote_modes() {
    let error = DesktopTauriLaunchOptions::from_args([
        "--remote-url",
        "https://deve.example",
        "--local-backend",
    ])
    .expect_err("conflicting mode must fail");

    assert_eq!(error, DesktopTauriLaunchOptionsError::ConflictingModes);
}

#[test]
fn desktop_launch_options_reject_missing_remote_url_value() {
    let error = DesktopTauriLaunchOptions::from_args(["--remote-url", "--local-backend"])
        .expect_err("missing url must fail");

    assert_eq!(error, DesktopTauriLaunchOptionsError::MissingRemoteUrlValue);
}

#[test]
fn desktop_launch_options_reject_invalid_remote_browser_url() {
    let error = DesktopTauriLaunchOptions::from_args(["--remote-url", "http://deve.example"])
        .expect_err("invalid url must fail");

    assert_eq!(error, DesktopTauriLaunchOptionsError::InvalidRemoteUrl);
}

#[test]
fn desktop_launch_options_support_manual_local_backend_disable() {
    let options = DesktopTauriLaunchOptions::from_args(["--no-local-backend"]).expect("options");

    assert_eq!(options.remote_url, None);
    assert_eq!(options.local_backend, Some(false));
}

#[test]
fn desktop_host_backend_preference_can_select_remote_browser() {
    let preference = NativeBackendPreference::remote("https://pref.example");

    let bootstrap = remote_browser_bootstrap_for_launch_options(
        &DesktopTauriLaunchOptions::default(),
        &preference,
    )
    .expect("bootstrap")
    .expect("remote bootstrap");

    assert!(bootstrap.source().contains("https://pref.example"));
}

#[test]
fn desktop_local_backend_option_overrides_remote_preference() {
    let preference = NativeBackendPreference::remote("https://pref.example");
    let options = DesktopTauriLaunchOptions {
        remote_url: None,
        local_backend: Some(true),
    };

    let bootstrap =
        remote_browser_bootstrap_for_launch_options(&options, &preference).expect("bootstrap");

    assert!(bootstrap.is_none());
}

#[test]
fn desktop_main_window_close_requests_process_exit() {
    assert!(desktop_main_window_close_exits_process(
        DESKTOP_TAURI_MAIN_WINDOW_LABEL
    ));
    assert!(!desktop_main_window_close_exits_process("secondary"));
}

#[test]
fn desktop_remote_env_overrides_host_backend_preference() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let _guard = EnvGuard::set(
        crate::DEVE_NATIVE_REMOTE_URL_ENV,
        Some("https://env.example"),
    );
    let preference = NativeBackendPreference::remote("https://pref.example");

    let bootstrap = remote_browser_bootstrap_for_launch_options(
        &DesktopTauriLaunchOptions::default(),
        &preference,
    )
    .expect("bootstrap")
    .expect("remote bootstrap");

    assert!(bootstrap.source().contains("https://env.example"));
    assert!(!bootstrap.source().contains("https://pref.example"));
}

#[test]
fn desktop_menu_actions_map_only_to_shell_effects() {
    assert_eq!(
        menu_action_shell_effect(DesktopMenuAction::ShowMainWindow),
        DesktopTauriShellEffect::ShowMainWindow
    );
    assert_eq!(
        menu_action_shell_effect(DesktopMenuAction::OpenCommandPalette),
        DesktopTauriShellEffect::ShowMainWindow
    );
    assert_eq!(
        menu_action_shell_effect(DesktopMenuAction::OpenSettings),
        DesktopTauriShellEffect::ShowMainWindow
    );
    assert_eq!(
        menu_action_shell_effect(DesktopMenuAction::QuitRequested),
        DesktopTauriShellEffect::QuitRequested
    );
}

#[test]
fn desktop_tray_actions_map_only_to_shell_effects() {
    assert_eq!(
        tray_action_shell_effect(DesktopTrayAction::ShowMainWindow),
        DesktopTauriShellEffect::ShowMainWindow
    );
    assert_eq!(
        tray_action_shell_effect(DesktopTrayAction::ToggleWindowVisibility),
        DesktopTauriShellEffect::ToggleMainWindowVisibility
    );
    assert_eq!(
        tray_action_shell_effect(DesktopTrayAction::QuitRequested),
        DesktopTauriShellEffect::QuitRequested
    );
}

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard {
    key: &'static str,
    old: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let old = std::env::var(key).ok();
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match self.old.as_ref() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
