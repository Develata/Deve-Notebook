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

#[test]
fn android_remote_recovery_uses_capability_free_activity_anchor_in_contract_order() {
    let manifest = include_str!("../../gen/android/app/src/main/AndroidManifest.xml");
    let anchor_activity = include_str!(
        "../../gen/android/app/src/main/java/dev/deve/notebook/mobile/RecoveryAnchorActivity.kt"
    );
    let restart_activity = include_str!(
        "../../gen/android/app/src/main/java/dev/deve/notebook/mobile/BackendRecoveryRestartActivity.kt"
    );
    let android_adapter = include_str!("backend_recovery/android.rs");
    let coordinator = include_str!("backend_recovery/coordinator.rs");
    let recovery_module = include_str!("backend_recovery/mod.rs");
    let native_commands = include_str!("native_backend_commands.rs");
    let proguard = include_str!("../../gen/android/app/proguard-rules.pro");
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../../capabilities/local-backend.json"))
            .expect("mobile capability");

    assert!(manifest.contains("android:name=\".RecoveryAnchorActivity\""));
    assert!(manifest.contains("android:exported=\"false\""));
    assert!(manifest.contains("android:excludeFromRecents=\"true\""));
    assert!(anchor_activity.contains("class RecoveryAnchorActivity : TauriActivity()"));
    assert!(anchor_activity.contains("scheduleBackendRecoveryColdStart"));
    assert!(anchor_activity.contains("BackendRecoveryRestartActivity::class.java"));
    assert!(anchor_activity.contains("addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)"));
    assert!(!anchor_activity.contains("Intent.makeRestartActivityTask"));
    assert!(restart_activity.contains("class BackendRecoveryRestartActivity : Activity()"));
    assert!(restart_activity.contains("activityManager.runningAppProcesses"));
    assert!(restart_activity.contains("REQUIRED_ABSENT_SAMPLES = 2"));
    assert!(restart_activity.contains("Intent.makeRestartActivityTask(component)"));
    assert!(manifest.contains("android:name=\".BackendRecoveryRestartActivity\""));
    assert!(manifest.contains("android:process=\":backend_recovery_restart\""));
    assert!(
        manifest.contains("android:taskAffinity=\"${applicationId}.backend_recovery_restart\"")
    );
    assert!(!anchor_activity.contains("invoke_handler"));
    assert!(!anchor_activity.contains("plugin"));
    assert_eq!(
        capability
            .pointer("/local")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        capability
            .pointer("/windows/0")
            .and_then(|value| value.as_str()),
        Some(MOBILE_TAURI_MAIN_WINDOW_LABEL)
    );
    assert_eq!(
        capability
            .pointer("/windows")
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(1)
    );
    assert!(native_commands.contains("ensure_bundled_local_origin(&window)?"));
    assert!(android_adapter.contains("WebviewUrl::External(url)"));
    assert!(android_adapter.contains(concat!("tauri", "::Url::parse(\"about:blank\")")));
    assert!(android_adapter.contains(".activity_name(ANDROID_RECOVERY_ANCHOR_ACTIVITY)"));
    assert!(android_adapter.contains(".created_by_activity_name(&main_activity_name)"));
    assert!(android_adapter.contains("scheduleBackendRecoveryColdStart"));
    assert!(proguard.contains("class dev.deve.notebook.mobile.RecoveryAnchorActivity"));
    assert!(proguard.contains("public boolean scheduleBackendRecoveryColdStart();"));
    assert!(!coordinator.contains("request_restart()"));
    assert!(android_adapter.contains(concat!("std::", "process::exit(exit_code)")));
    assert!(recovery_module.contains(concat!("android::", "retire_process(exit_code)")));
    assert!(!recovery_module.contains(concat!("std::", "process")));
    assert!(!recovery_module.contains("app.exit(exit_code)"));

    let completion_failure = &coordinator[coordinator
        .find("if let Err(error) = coordinator.recovery.finish_success(recovery_id)")
        .expect("completion failure path")..];
    let managed_shutdown = completion_failure
        .find("shutdown_managed_supervisor")
        .expect("managed supervisor shutdown");
    let forced_restart = completion_failure
        .find("request_platform_cold_restart")
        .expect("forced restart");
    assert!(managed_shutdown < forced_restart);

    let flow = &coordinator[coordinator
        .find("async fn switch_to_local")
        .expect("switch flow")..];
    let anchor_create = flow
        .find("create_platform_recovery_anchor")
        .expect("anchor creation");
    let control_retire = flow
        .find("remove_platform_recovery_control")
        .expect("control retirement");
    let remote_retire = flow
        .find("retire_platform_remote_surface")
        .expect("remote retirement");
    let preference_commit = flow
        .find("save_preference(NativeBackendPreference::local())")
        .expect("preference commit");
    let plugin_registration = flow
        .find("self.app.plugin(mobile_local_backend_command_plugin())")
        .expect("plugin registration");
    let local_create = flow
        .find("create_platform_local_main_window")
        .expect("local creation");
    let anchor_retire = flow
        .find("retire_and_confirm_recovery_anchor")
        .expect("anchor retirement");
    let local_record = flow
        .find("MobileBackendRecoveryPhase::LocalWindowCreated")
        .expect("local record");

    assert!(anchor_create < control_retire);
    assert!(control_retire < remote_retire);
    assert!(remote_retire < preference_commit);
    assert!(preference_commit < plugin_registration);
    assert!(plugin_registration < local_create);
    assert!(local_create < anchor_retire);
    assert!(anchor_retire < local_record);
}
