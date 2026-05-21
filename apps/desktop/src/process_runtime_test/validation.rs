use std::path::PathBuf;

use deve_core::native_adapter::{
    NativeProcessRuntimeError, NativeProcessRuntimeFailureKind, NativeProcessRuntimeState,
};

use crate::process_runtime::{DesktopLocalServiceRuntime, DesktopProcessRuntimeError};

use super::support::{RecordingLauncher, enabled_policy, valid_spawn_spec};

#[test]
fn desktop_local_service_runtime_rejects_invalid_spawn_spec() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.executable = PathBuf::from("deve_cli");

    let error = runtime.start(&spec, 1).expect_err("invalid spawn spec");
    assert!(matches!(
        error,
        DesktopProcessRuntimeError::Contract(NativeProcessRuntimeError::RelativePathForbidden {
            field: "executable"
        })
    ));
    assert_eq!(
        runtime.snapshot().state,
        NativeProcessRuntimeState::Disabled
    );
    assert!(runtime.events().is_empty());
}

#[test]
fn desktop_local_service_runtime_rejects_non_deve_cli_command_before_spawn() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.executable = std::env::current_dir()
        .expect("current dir")
        .join("target/native/other_tool");

    let error = runtime.start(&spec, 10).expect_err("reject command");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "executable must be deve_cli"
        }
    ));
    assert_eq!(runtime.snapshot().state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        runtime.snapshot().last_failure,
        Some(NativeProcessRuntimeFailureKind::InvalidExecutablePath)
    );
}

#[test]
fn desktop_local_service_runtime_rejects_non_serve_argv_before_spawn() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.argv = vec!["dump".to_string()];

    let error = runtime.start(&spec, 10).expect_err("reject argv");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "first argv must be serve"
        }
    ));
}

#[test]
fn desktop_local_service_runtime_rejects_extra_serve_argv_before_spawn() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.argv.push("--dev".to_string());

    let error = runtime.start(&spec, 10).expect_err("reject extra argv");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv must be exactly serve --native-loopback --port <port>"
        }
    ));
}

#[test]
fn desktop_local_service_runtime_rejects_mismatched_argv_port_before_spawn() {
    let launcher = RecordingLauncher::default();
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    let mut spec = valid_spawn_spec();
    spec.argv[3] = "39101".to_string();

    let error = runtime.start(&spec, 10).expect_err("reject port mismatch");

    assert!(matches!(
        error,
        DesktopProcessRuntimeError::InvalidServiceCommand {
            reason: "argv port must match loopback bind hints"
        }
    ));
}
