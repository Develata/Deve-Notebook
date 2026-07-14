use super::*;

#[test]
fn mobile_tauri_runtime_surface_is_shell_only() {
    let surface = mobile_tauri_runtime_surface();

    assert!(surface.is_shell_only());
    assert!(surface.android_shell_package_entrypoint_declared);
    assert!(surface.ios_shell_package_entrypoint_declared);
    assert!(surface.build_script_declared);
    assert!(surface.webview_shell_runtime_declared);
    assert!(surface.local_backend_default_enabled);
    assert!(surface.embedded_service_runtime_enabled);
    assert!(!surface.child_process_runtime_enabled);
    assert!(!surface.opens_authority_write_path);
    assert!(!surface.release_ready_claimed);
}

#[test]
fn mobile_tauri_remote_browser_resolves_https_target_without_init_script() {
    let mode = mobile_tauri_mode_for_inputs(
        &MobileTauriLaunchOptions {
            remote_url: Some("https://deve.example".to_string()),
            local_backend: None,
        },
        &NativeBackendPreference::local(),
        None,
    )
    .expect("remote target");

    assert_eq!(
        mode.remote_target.expect("remote mode").https_origin,
        "https://deve.example"
    );
    assert!(!mode.native_local_recovery_control);
}

#[test]
fn mobile_tauri_remote_browser_rejects_non_https_origin() {
    let error = mobile_tauri_mode_for_inputs(
        &MobileTauriLaunchOptions {
            remote_url: Some("http://deve.example".to_string()),
            local_backend: None,
        },
        &NativeBackendPreference::local(),
        None,
    )
    .expect_err("http remote target must fail");

    assert!(matches!(
        error,
        MobileTauriModeError::RemoteTarget(NativeAdapterError::WrongScheme {
            expected_scheme: "https",
            ..
        })
    ));
}

#[test]
fn mobile_launch_options_parse_remote_browser_url() {
    let options = MobileTauriLaunchOptions::from_args(["--remote-url", "https://deve.example"])
        .expect("options");

    assert_eq!(options.remote_url.as_deref(), Some("https://deve.example"));
    assert_eq!(options.local_backend, None);
}

#[test]
fn mobile_launch_options_reject_conflicting_local_and_remote_modes() {
    let error = MobileTauriLaunchOptions::from_args([
        "--remote-url",
        "https://deve.example",
        "--local-backend",
    ])
    .expect_err("conflicting mode must fail");

    assert_eq!(error, MobileTauriLaunchOptionsError::ConflictingModes);

    let error = mobile_tauri_mode_for_inputs(
        &MobileTauriLaunchOptions {
            remote_url: Some("https://deve.example".to_string()),
            local_backend: Some(true),
        },
        &NativeBackendPreference::local(),
        None,
    )
    .expect_err("constructed conflicting mode must fail");
    assert!(matches!(error, MobileTauriModeError::ConflictingModes));
}

#[test]
fn mobile_host_backend_preference_can_select_remote_browser() {
    let preference = NativeBackendPreference::remote("https://pref.example");

    let mode =
        mobile_tauri_mode_for_inputs(&MobileTauriLaunchOptions::default(), &preference, None)
            .expect("target");

    assert_eq!(
        mode.remote_target.expect("remote target").https_origin,
        "https://pref.example"
    );
    assert!(mode.native_local_recovery_control);
}

#[test]
fn mobile_local_backend_option_overrides_remote_preference() {
    let preference = NativeBackendPreference::remote("https://pref.example");
    let options = MobileTauriLaunchOptions {
        remote_url: None,
        local_backend: Some(true),
    };

    let mode = mobile_tauri_mode_for_inputs(&options, &preference, None).expect("target");

    assert!(mode.remote_target.is_none());
    assert!(!mode.native_local_recovery_control);
}

#[test]
fn mobile_remote_env_overrides_host_backend_preference() {
    let preference = NativeBackendPreference::remote("https://pref.example");

    let mode = mobile_tauri_mode_for_inputs(
        &MobileTauriLaunchOptions::default(),
        &preference,
        Some(NativeRemoteTarget {
            https_origin: "https://env.example".to_string(),
        }),
    )
    .expect("target");

    assert_eq!(
        mode.remote_target.expect("remote target").https_origin,
        "https://env.example"
    );
    assert!(!mode.native_local_recovery_control);
}

#[test]
fn mobile_tauri_main_window_creation_is_deferred_until_bootstrap() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).expect("tauri config");
    assert_eq!(
        config
            .pointer("/app/windows/0/label")
            .and_then(|value| value.as_str()),
        Some(MOBILE_TAURI_MAIN_WINDOW_LABEL)
    );
    assert_eq!(
        config
            .pointer("/app/windows/0/create")
            .and_then(|value| value.as_bool()),
        Some(false)
    );
}
