//! plan_ref: infra

use anyhow::{Result, bail};
use std::env;
use std::process::ExitCode;

mod acceptance_matrix;
mod ai;
mod architecture_registry;
mod auth;
mod auth_unauthorized_state;
mod backup;
mod browser_prefs_boundary;
mod cargo_gate;
mod cargo_test;
mod cli_settings;
mod context;
mod desktop_package_preflight;
mod desktop_signing_preflight;
mod desktop_target_host_preflight;
mod dev_data_health;
mod dev_runbook;
mod diff_color;
mod docker_smoke_preflight;
mod env_gate;
mod feature_operation_paths;
mod foundation;
mod graph;
mod i18n_formatting;
mod i18n_hardcoded;
mod large_doc;
mod mobile;
mod mobile_android_install_startup_smoke;
mod mobile_android_release_preflight;
mod mobile_android_shell_package_build;
mod mobile_ios_install_startup_smoke;
mod mobile_ios_shell_package_build;
mod mobile_platform_package_preflight;
mod mobile_shell_gate;
mod native_packaging_gate;
mod native_process_adapter_gate;
mod native_target_host_evidence;
mod native_track_boundary;
mod network;
mod perf_budget;
mod release;
mod release_audit_gate;
mod release_candidate;
mod release_freeze;
mod release_version_order;
mod reliability_observability;
mod remote_fixture_password;
mod rendering;
mod repo_file_ops;
mod script_gate_preflight;
mod search;
mod settings_local_feedback;
mod source_control;
mod source_control_smoke_hygiene;
mod spec;
mod storage_repo;
mod ui_dashboard_refresh;
mod ui_desktop;
mod ui_disconnect;
mod ui_focus;
mod ui_spa_routing;
mod ui_token;
mod ui_z_index;
mod web_runtime_boundary;
mod workspace_root;
mod ws_structured_errors;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "--help".to_string());
    let command_args: Vec<String> = args.collect();

    match command.as_str() {
        "storage-repo" => storage_repo::run(),
        "acceptance-matrix" => acceptance_matrix::run(&command_args),
        "acceptance-receipt" => acceptance_matrix::run_receipt(&command_args),
        "acceptance-run" => acceptance_matrix::run_producers(&command_args),
        "acceptance-collect" => acceptance_matrix::collect_receipts(&command_args),
        "acceptance-impact" => acceptance_matrix::run_impact(&command_args),
        "architecture-registry" => architecture_registry::run(),
        "network" => network::run(),
        "release" => release::run(),
        "release-candidate" => release_candidate::run(&command_args),
        "release-freeze" => release_freeze::run(&command_args),
        "release-version-order" => release_version_order::run(&command_args),
        "remote-fixture-password-hash" => remote_fixture_password::run(&command_args),
        "dev-runbook" => dev_runbook::run(),
        "diff-color" => diff_color::run(),
        "feature-operation-paths" => feature_operation_paths::run(),
        "graph" => graph::run(),
        "i18n-formatting" => i18n_formatting::run(),
        "i18n-hardcoded" => i18n_hardcoded::run(),
        "rendering" => rendering::run(),
        "search" => search::run(),
        "ui-token" => ui_token::run(),
        "ui-z-index" => ui_z_index::run(),
        "ui-focus" => ui_focus::run(),
        "auth" => auth::run(),
        "auth-unauthorized-state" => auth_unauthorized_state::run(),
        "backup" => backup::run(),
        "browser-prefs-boundary" => browser_prefs_boundary::run(),
        "ai" => ai::run(),
        "cli-settings" => cli_settings::run(),
        "dev-data-health" => dev_data_health::run(),
        "deep-audit-gate" => script_gate_preflight::run_deep_audit_gate(),
        "docker-smoke-preflight" => docker_smoke_preflight::run(&command_args),
        "desktop-package-preflight" => desktop_package_preflight::run(),
        "desktop-signing-preflight" => desktop_signing_preflight::run(),
        "desktop-target-host-preflight" => desktop_target_host_preflight::run(),
        "desktop-platform-package-build" => {
            script_gate_preflight::run_desktop_platform_package_build()
        }
        "desktop-package-startup-smoke" => {
            script_gate_preflight::run_desktop_package_startup_smoke()
        }
        "desktop-native-session-package-smoke" => {
            script_gate_preflight::run_desktop_native_session_package_smoke()
        }
        "desktop-installer-smoke" => script_gate_preflight::run_desktop_installer_smoke(),
        "foundation" => foundation::run(),
        "large-doc" => large_doc::run(),
        "local-quick-gate" => script_gate_preflight::run_local_quick_gate(),
        "mobile" => mobile::run(),
        "mobile-android-release-preflight" => mobile_android_release_preflight::run(),
        "mobile-android-emulator-install-startup-smoke" => {
            script_gate_preflight::run_mobile_android_emulator_install_startup_smoke()
        }
        "mobile-android-install-startup-smoke" => mobile_android_install_startup_smoke::run(),
        "mobile-android-shell-package-build" => mobile_android_shell_package_build::run(),
        "mobile-ios-install-startup-smoke" => mobile_ios_install_startup_smoke::run(),
        "mobile-ios-shell-package-build" => mobile_ios_shell_package_build::run(),
        "mobile-platform-package-preflight" => mobile_platform_package_preflight::run(),
        "native-packaging-gate" => native_packaging_gate::run(),
        "native-process-adapter-gate" => native_process_adapter_gate::run(),
        "native-track-boundary" => native_track_boundary::run(),
        "native-target-host-evidence" => native_target_host_evidence::run(&command_args),
        "perf-budget" => perf_budget::run(),
        "reliability-observability" => reliability_observability::run(),
        "release-audit-gate" => release_audit_gate::run(&command_args),
        "repo-file-ops" => repo_file_ops::run(),
        "settings-local-feedback" => settings_local_feedback::run(),
        "source-control" => source_control::run(),
        "source-control-smoke-hygiene" => source_control_smoke_hygiene::run(),
        "ui-dashboard-refresh" => ui_dashboard_refresh::run(),
        "ui-desktop" => ui_desktop::run(),
        "ui-disconnect" => ui_disconnect::run(),
        "ui-spa-routing" => ui_spa_routing::run(),
        "web-runtime-boundary" => web_runtime_boundary::run(),
        "ws-structured-errors" => ws_structured_errors::run(),
        "all" => run_text_baselines(),
        "full" => run_full_baselines(),
        "-h" | "--help" | "help" => {
            println!(
                "Usage: deve_baseline <storage-repo|acceptance-matrix|acceptance-receipt|acceptance-run|acceptance-collect|acceptance-impact|architecture-registry|network|release|release-candidate|release-freeze|release-version-order|remote-fixture-password-hash|dev-runbook|diff-color|feature-operation-paths|graph|i18n-formatting|i18n-hardcoded|rendering|search|ui-token|ui-z-index|ui-focus|auth|auth-unauthorized-state|backup|browser-prefs-boundary|ai|cli-settings|dev-data-health|deep-audit-gate|docker-smoke-preflight|desktop-package-preflight|desktop-signing-preflight|desktop-target-host-preflight|desktop-platform-package-build|desktop-package-startup-smoke|desktop-native-session-package-smoke|desktop-installer-smoke|foundation|large-doc|local-quick-gate|mobile|mobile-android-release-preflight|mobile-android-emulator-install-startup-smoke|mobile-android-install-startup-smoke|mobile-android-shell-package-build|mobile-ios-install-startup-smoke|mobile-ios-shell-package-build|mobile-platform-package-preflight|native-packaging-gate|native-process-adapter-gate|native-track-boundary|native-target-host-evidence|perf-budget|reliability-observability|release-audit-gate|repo-file-ops|settings-local-feedback|source-control|source-control-smoke-hygiene|ui-dashboard-refresh|ui-desktop|ui-disconnect|ui-spa-routing|web-runtime-boundary|ws-structured-errors|all|full>"
            );
            Ok(())
        }
        other => {
            bail!(
                "unknown baseline check '{other}'. run `deve_baseline --help` for the supported checks"
            )
        }
    }
}

fn run_text_baselines() -> Result<()> {
    storage_repo::run()?;
    acceptance_matrix::run(&[])?;
    architecture_registry::run()?;
    network::run()?;
    release::run()?;
    release_freeze::verify()?;
    dev_runbook::run()?;
    diff_color::run()?;
    feature_operation_paths::run()?;
    graph::run()?;
    i18n_formatting::run()?;
    i18n_hardcoded::run()?;
    rendering::run()?;
    search::run()?;
    ui_token::run()?;
    ui_z_index::run()?;
    ui_focus::run()?;
    auth::run_text()?;
    auth_unauthorized_state::run()?;
    backup::run_text()?;
    browser_prefs_boundary::run()?;
    ai::run_text()?;
    cli_settings::run_text()?;
    dev_data_health::run_text()?;
    foundation::run_text()?;
    large_doc::run_text()?;
    mobile::run_text()?;
    native_track_boundary::run()?;
    native_target_host_evidence::run(&[])?;
    perf_budget::run_text()?;
    reliability_observability::run_text()?;
    settings_local_feedback::run_text()?;
    source_control::run_text()?;
    source_control_smoke_hygiene::run()?;
    ui_dashboard_refresh::run_text()?;
    ui_desktop::run_text()?;
    ui_disconnect::run_text()?;
    ui_spa_routing::run_text()?;
    web_runtime_boundary::run_text()?;
    ws_structured_errors::run()
}

fn run_full_baselines() -> Result<()> {
    storage_repo::run()?;
    acceptance_matrix::run(&[])?;
    architecture_registry::run()?;
    network::run()?;
    release::run()?;
    release_freeze::verify()?;
    dev_runbook::run()?;
    diff_color::run()?;
    feature_operation_paths::run()?;
    graph::run()?;
    i18n_formatting::run()?;
    i18n_hardcoded::run()?;
    rendering::run()?;
    search::run()?;
    ui_token::run()?;
    ui_z_index::run()?;
    ui_focus::run()?;
    auth::run()?;
    auth_unauthorized_state::run()?;
    backup::run()?;
    browser_prefs_boundary::run()?;
    ai::run()?;
    cli_settings::run()?;
    dev_data_health::run()?;
    desktop_package_preflight::run()?;
    foundation::run()?;
    large_doc::run()?;
    mobile::run()?;
    native_packaging_gate::run()?;
    native_process_adapter_gate::run()?;
    native_target_host_evidence::run(&[])?;
    perf_budget::run()?;
    reliability_observability::run()?;
    repo_file_ops::run()?;
    settings_local_feedback::run()?;
    source_control::run()?;
    source_control_smoke_hygiene::run()?;
    ui_dashboard_refresh::run()?;
    ui_desktop::run()?;
    ui_disconnect::run()?;
    ui_spa_routing::run()?;
    web_runtime_boundary::run()?;
    ws_structured_errors::run()
}
