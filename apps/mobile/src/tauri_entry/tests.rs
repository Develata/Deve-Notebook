use super::*;

mod android_presentation;
mod android_recovery_control;

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
fn android_backend_recovery_uses_capability_free_activity_anchor_in_contract_order() {
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
    let handoff_defer = flow
        .find("defer_initial_webview_session_for_recovery")
        .expect("WebView handoff admission defer");
    let handoff_admit = flow
        .find("admit_initial_webview_session_after_recovery")
        .expect("WebView handoff admission grant");

    assert!(handoff_defer < anchor_create);
    assert!(anchor_create < control_retire);
    assert!(control_retire < remote_retire);
    assert!(remote_retire < preference_commit);
    assert!(preference_commit < plugin_registration);
    assert!(plugin_registration < local_create);
    assert!(local_create < anchor_retire);
    assert!(anchor_retire < local_record);
    assert!(local_record < handoff_admit);
}

#[test]
fn android_backend_recovery_initial_session_uses_invoking_current_webview() {
    let commands = include_str!("native_backend_commands.rs");
    let prepare = &commands[commands
        .find("async fn native_backend_prepare_webview_session")
        .expect("initial session command")..];
    let prepare = &prepare[..prepare
        .find("async fn native_backend_debug_stop_transport")
        .expect("next native command")];

    assert!(prepare.contains("state.prepare_initial_webview_session(&window).await"));
    assert!(!prepare.contains("get_webview_window"));
    assert!(!prepare.contains("MOBILE_MAIN_WINDOW_LABEL"));
}

#[test]
fn android_back_dispatch_prioritizes_ime_and_backgrounds_only_after_matching_unhandled_ack() {
    let activity = include_str!(
        "../../gen/android/app/src/main/java/dev/deve/notebook/mobile/MainActivity.kt"
    );
    let back_dispatcher = include_str!(
        "../../gen/android/app/src/main/java/dev/deve/notebook/mobile/UiBackDispatcher.kt"
    );

    assert!(activity.contains("uiBackDispatcher.install()"));
    assert!(activity.contains("uiBackDispatcher.attach(webView)"));
    assert!(activity.contains("uiBackDispatcher.detach()"));
    assert!(back_dispatcher.contains("OnBackPressedCallback(true)"));
    assert!(back_dispatcher.contains("if (!dismissVisibleIme()) requestUiBack()"));
    assert!(back_dispatcher.contains("WindowInsetsCompat.Type.ime()"));
    assert!(back_dispatcher.contains("android_ui_back_ime_dismissed"));
    assert!(back_dispatcher.contains("android_ui_back_ime_dismiss_failed"));
    assert!(back_dispatcher.contains("android_ui_back_ime_visibility_unavailable"));
    assert!(back_dispatcher.contains("requestUiBack()"));
    assert!(back_dispatcher.contains("deve-native-back-request"));
    assert!(back_dispatcher.contains("const detail = { requestId:"));
    assert!(back_dispatcher.contains("listenerSeen: false"));
    assert!(back_dispatcher.contains("ack.optString(\"requestId\") != requestId.toString()"));
    assert!(back_dispatcher.contains("\"Unhandled\" ->"));
    assert!(back_dispatcher.contains("activeRequestId == requestId"));
    assert!(back_dispatcher.contains("activeRequestId = null"));
    assert!(back_dispatcher.contains("webViewGeneration"));
    assert!(back_dispatcher.contains("requestIsCurrent"));
    assert!(back_dispatcher.contains("retireActiveRequest()"));
    assert!(back_dispatcher.contains("android_ui_back_handled"));
    assert!(back_dispatcher.contains("android_ui_back_root_backgrounded"));
    assert!(back_dispatcher.contains("android_ui_back_background_failed"));
    assert!(back_dispatcher.contains("android_ui_back_ack_timeout"));
    assert!(back_dispatcher.contains("android_ui_back_listener_missing"));
    assert!(back_dispatcher.contains("activity.moveTaskToBack(true)"));
    assert!(!back_dispatcher.contains("finish()"));
    assert!(!back_dispatcher.contains("webView.goBack()"));
    assert!(!back_dispatcher.contains("webView.canGoBack()"));

    let timeout = &back_dispatcher[back_dispatcher
        .find("val timeout = Runnable")
        .expect("back acknowledgement timeout")..];
    let evaluate = timeout
        .find("webView.evaluateJavascript")
        .expect("typed WebView dispatch");
    assert!(!timeout[..evaluate].contains("finish()"));

    let matching_ack = &back_dispatcher[back_dispatcher
        .find("ack.optString(\"requestId\") != requestId.toString()")
        .expect("matching acknowledgement guard")..];
    let unhandled = matching_ack
        .find("\"Unhandled\" ->")
        .expect("Unhandled acknowledgement");
    let background = matching_ack[unhandled..]
        .find("moveTaskToBack(true)")
        .expect("root task background after matching Unhandled acknowledgement");
    assert!(background > 0);
}

#[test]
fn android_release_shrinker_rule_keeps_wry_activity_jni_id_getter() {
    let proguard = include_str!("../../gen/android/app/proguard-rules.pro");

    assert!(proguard.contains("class dev.deve.notebook.mobile.WryActivity"));
    assert!(proguard.contains("public int getId();"));
}

#[test]
fn android_release_variant_enables_minification_and_proguard_rules() {
    let gradle = include_str!("../../gen/android/app/build.gradle.kts");
    let release = gradle
        .split_once("getByName(\"release\")")
        .expect("Android release build type")
        .1;

    assert!(release.contains("isMinifyEnabled = true"));
    assert!(release.contains("proguardFiles("));
    assert!(release.contains("proguard-android-optimize.txt"));
}

#[test]
fn android_release_cleartext_is_scoped_to_exact_loopback_destinations() {
    let main_manifest = include_str!("../../gen/android/app/src/main/AndroidManifest.xml");
    let release_manifest = include_str!("../../gen/android/app/src/release/AndroidManifest.xml");
    let network_security =
        include_str!("../../gen/android/app/src/release/res/xml/network_security_config.xml");

    assert!(main_manifest.contains("android:usesCleartextTraffic=\"${usesCleartextTraffic}\""));
    assert!(
        release_manifest.contains("android:networkSecurityConfig=\"@xml/network_security_config\"")
    );
    assert!(network_security.contains("<base-config cleartextTrafficPermitted=\"false\" />"));
    assert!(network_security.contains("<domain includeSubdomains=\"false\">127.0.0.1</domain>"));
    assert!(network_security.contains("<domain includeSubdomains=\"false\">localhost</domain>"));
    assert_eq!(
        network_security
            .matches("cleartextTrafficPermitted=\"true\"")
            .count(),
        1
    );
    assert_eq!(
        network_security
            .matches("includeSubdomains=\"false\"")
            .count(),
        2
    );
    assert!(!network_security.contains("includeSubdomains=\"true\""));
}
