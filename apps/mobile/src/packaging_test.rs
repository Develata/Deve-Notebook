//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate

use serde_json::Value;

use crate::{
    MOBILE_ANDROID_PACKAGE_GATE_ANCHOR, MOBILE_ANDROID_PACKAGE_SCRIPT,
    MOBILE_IOS_PACKAGE_GATE_ANCHOR, MOBILE_IOS_PACKAGE_SCRIPT, MOBILE_TAURI_CONFIG_PATH,
    MOBILE_TAURI_IDENTIFIER, MOBILE_TAURI_MAIN_WINDOW_LABEL, MOBILE_TAURI_MAIN_WINDOW_TITLE,
    MOBILE_TAURI_PRODUCT_NAME, MobilePackagingAuthority, MobilePackagingCapability,
    mobile_packaging_scaffold, mobile_tauri_runtime_surface,
};
use deve_core::native_adapter::CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

#[test]
fn mobile_packaging_dependency_spike_is_feature_gated() {
    let scaffold = mobile_packaging_scaffold();
    let gate = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(scaffold.dependency_batch.feature_gate, "native-packaging");
    assert_eq!(scaffold.dependency_batch.runtime_crate, "tauri");
    assert_eq!(scaffold.dependency_batch.build_crate, "tauri-build");
    assert_eq!(scaffold.dependency_batch.status, "dependency-spike-open");
    assert!(scaffold.dependency_feature_is_isolated());
    assert!(gate.is_mobile_dependency_spike_open());
    assert!(gate.mobile_tauri_dependencies_allowed);
    assert!(!gate.mobile_packaging_stays_deferred());
}

#[test]
fn mobile_packaging_acceptance_is_shell_only() {
    let scaffold = mobile_packaging_scaffold();

    assert_eq!(
        scaffold.acceptance.capabilities,
        [
            MobilePackagingCapability::WebViewShell,
            MobilePackagingCapability::PermissionBridge,
            MobilePackagingCapability::ShareSheet,
            MobilePackagingCapability::DeepLink,
            MobilePackagingCapability::FilePicker,
            MobilePackagingCapability::PushNotification,
            MobilePackagingCapability::StorePackage,
        ]
    );
    assert_eq!(
        scaffold.acceptance.forbidden_authorities,
        [
            MobilePackagingAuthority::Ledger,
            MobilePackagingAuthority::Vault,
            MobilePackagingAuthority::SourceControl,
            MobilePackagingAuthority::SearchIndex,
            MobilePackagingAuthority::GitMirror,
            MobilePackagingAuthority::NoteGit,
        ]
    );
    assert!(scaffold.is_authority_free());
    assert!(scaffold.shell_acceptance_is_authority_free());
    assert!(
        scaffold
            .acceptance
            .android_shell_package
            .is_shell_only_open()
    );
    assert!(scaffold.acceptance.ios_shell_package.is_shell_only_open());
    assert!(scaffold.acceptance.lifecycle_reprobe_remains_required);
    assert!(scaffold.no_packaging_tests_remain_authoritative);
}

#[test]
fn mobile_tauri_manifest_declares_shell_metadata_only() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json"))
        .expect("mobile tauri config should be valid json");
    let scaffold = mobile_packaging_scaffold();
    let shell = scaffold.acceptance.shell;
    let window = &config["app"]["windows"][0];

    assert_eq!(shell.tauri_config_path, MOBILE_TAURI_CONFIG_PATH);
    assert_eq!(config["productName"], MOBILE_TAURI_PRODUCT_NAME);
    assert_eq!(config["identifier"], MOBILE_TAURI_IDENTIFIER);
    assert_eq!(config["build"]["devUrl"], "http://127.0.0.1:3001");
    assert_eq!(config["build"]["frontendDist"], "../web/dist");
    assert_eq!(window["label"], MOBILE_TAURI_MAIN_WINDOW_LABEL);
    assert_eq!(window["title"], MOBILE_TAURI_MAIN_WINDOW_TITLE);
    assert_eq!(window["width"], 390);
    assert_eq!(window["height"], 844);
    assert_eq!(window["minWidth"], 320);
    assert_eq!(window["minHeight"], 568);
    assert_eq!(window["resizable"], true);
    assert_eq!(window["fullscreen"], false);
    assert_eq!(config["app"]["withGlobalTauri"], false);
    assert_eq!(config["app"]["security"]["csp"], Value::Null);
    assert_eq!(config["bundle"]["active"], true);
    assert_eq!(
        config["bundle"]["icon"],
        serde_json::json!(["icons/icon.png", "icons/icon.ico", "icons/icon.icns"])
    );
    assert_eq!(config["bundle"]["createUpdaterArtifacts"], false);
    assert!(
        config["plugins"]
            .as_object()
            .is_some_and(|plugins| plugins.is_empty())
    );
}

#[test]
fn mobile_shell_acceptance_keeps_runtime_authority_closed() {
    let shell = mobile_packaging_scaffold().acceptance.shell;

    assert_eq!(shell.product_name, MOBILE_TAURI_PRODUCT_NAME);
    assert_eq!(shell.identifier, MOBILE_TAURI_IDENTIFIER);
    assert_eq!(shell.main_window_label, MOBILE_TAURI_MAIN_WINDOW_LABEL);
    assert_eq!(shell.main_window_title, MOBILE_TAURI_MAIN_WINDOW_TITLE);
    assert!(shell.manifest_declared);
    assert!(shell.build_script_declared);
    assert!(shell.android_project_generated);
    assert!(!shell.ios_project_generated);
    assert!(shell.runtime_entrypoint_declared);
    assert!(shell.platform_package_build_declared);
    assert!(shell.session_handoff_required_before_writable_ui);
    assert!(shell.foreground_reprobe_required);
    assert!(!shell.child_process_runtime_enabled);
    assert!(!shell.release_ready_claimed);
}

#[test]
fn mobile_android_shell_package_gate_is_shell_only() {
    let android = mobile_packaging_scaffold().acceptance.android_shell_package;
    let runtime = mobile_tauri_runtime_surface();

    assert_eq!(android.gate_anchor, MOBILE_ANDROID_PACKAGE_GATE_ANCHOR);
    assert_eq!(android.target_host_script, MOBILE_ANDROID_PACKAGE_SCRIPT);
    assert!(android.project_generation_allowed);
    assert!(android.package_build_allowed);
    assert!(!android.ios_package_build_allowed);
    assert!(!android.child_process_runtime_enabled);
    assert!(!android.opens_authority_write_path);
    assert!(!android.release_ready_claimed);
    assert!(android.is_shell_only_open());
    assert!(runtime.is_shell_only());
}

#[test]
fn mobile_ios_shell_package_gate_is_shell_only() {
    let ios = mobile_packaging_scaffold().acceptance.ios_shell_package;
    let runtime = mobile_tauri_runtime_surface();

    assert_eq!(ios.gate_anchor, MOBILE_IOS_PACKAGE_GATE_ANCHOR);
    assert_eq!(ios.target_host_script, MOBILE_IOS_PACKAGE_SCRIPT);
    assert!(ios.project_generation_allowed);
    assert!(ios.package_build_allowed);
    assert!(!ios.android_package_build_allowed);
    assert!(!ios.child_process_runtime_enabled);
    assert!(!ios.opens_authority_write_path);
    assert!(!ios.release_ready_claimed);
    assert!(ios.is_shell_only_open());
    assert!(runtime.is_shell_only());
}
