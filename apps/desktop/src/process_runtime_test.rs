use std::path::PathBuf;

use deve_core::native_adapter::{
    NativeEndpointReady, NativeProcessBindHints, NativeProcessEnvBinding, NativeProcessExitStatus,
    NativeProcessPathResolution, NativeProcessRuntimeError, NativeProcessRuntimeFailureKind,
    NativeProcessRuntimeHandle, NativeProcessRuntimeState, NativeProcessSpawnSpec,
    NativeRuntimeReadiness, NativeServiceHealthProbe,
};

use crate::process_runtime::DesktopFakeProcessRuntime;
use crate::{DesktopServiceState, DesktopSessionMaterial, DesktopShell};

fn valid_spawn_spec() -> NativeProcessSpawnSpec {
    let root = std::env::current_dir().expect("current dir");
    NativeProcessSpawnSpec {
        executable: root.join("target/native/deve_cli"),
        argv: vec!["serve".to_string(), "--dev".to_string()],
        cwd: root.clone(),
        env_allowlist: vec!["DEVE_PROFILE".to_string()],
        env: vec![NativeProcessEnvBinding {
            key: "DEVE_PROFILE".to_string(),
            value: "standard".to_string(),
        }],
        profile: "standard".to_string(),
        config_path: root.join("config.toml"),
        vault_path: root.join("vault"),
        ledger_path: root.join("ledger"),
        bind_hints: NativeProcessBindHints {
            http_host: "127.0.0.1".to_string(),
            http_port: Some(3001),
            ws_host: "localhost".to_string(),
            ws_port: Some(3001),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    }
}

fn handle() -> NativeProcessRuntimeHandle {
    NativeProcessRuntimeHandle {
        handle_id: "fake-child-1".to_string(),
        platform_pid: Some(4242),
    }
}

fn endpoint(session_bound: bool) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound,
    }
}

fn healthy_probe() -> NativeServiceHealthProbe {
    NativeServiceHealthProbe {
        endpoint_reachable: true,
        node_role_readable: true,
    }
}

#[test]
fn desktop_process_runtime_fake_blocks_when_policy_closed() {
    let mut runtime = DesktopFakeProcessRuntime::disabled();

    assert_eq!(
        runtime.request_start(&valid_spawn_spec(), 1),
        Err(NativeProcessRuntimeError::RuntimeDisabled)
    );
    assert_eq!(
        runtime.snapshot().state,
        NativeProcessRuntimeState::Disabled
    );
    assert!(runtime.events().is_empty());
}

#[test]
fn desktop_process_runtime_fake_rejects_invalid_spawn_spec() {
    let mut runtime = DesktopFakeProcessRuntime::enabled_for_test(1);
    let mut spec = valid_spawn_spec();
    spec.executable = PathBuf::from("deve_cli");

    assert_eq!(
        runtime.request_start(&spec, 1),
        Err(NativeProcessRuntimeError::RelativePathForbidden {
            field: "executable"
        })
    );
    assert_eq!(
        runtime.snapshot().state,
        NativeProcessRuntimeState::Disabled
    );
    assert!(runtime.events().is_empty());
}

#[test]
fn desktop_process_runtime_fake_records_successful_state_sequence() {
    let mut runtime = DesktopFakeProcessRuntime::enabled_for_test(1);

    runtime
        .request_start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_started(handle(), 2);
    runtime.record_endpoint_probe(endpoint(false), healthy_probe(), 3);
    runtime.record_session_handoff(true, 4);
    runtime.mark_runtime_ready(5);

    let states: Vec<_> = runtime.events().iter().map(|event| event.state).collect();
    assert_eq!(
        states,
        [
            NativeProcessRuntimeState::SpawnRequested,
            NativeProcessRuntimeState::Spawned,
            NativeProcessRuntimeState::EndpointHealthy,
            NativeProcessRuntimeState::SessionHandoffReady,
            NativeProcessRuntimeState::RuntimeReady,
        ]
    );
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.state, NativeProcessRuntimeState::RuntimeReady);
    assert!(snapshot.handle.is_some());
    assert_eq!(snapshot.endpoint.expect("endpoint").session_bound, true);
}

#[test]
fn desktop_process_runtime_fake_probe_failure_consumes_retry_budget() {
    let mut runtime = DesktopFakeProcessRuntime::enabled_for_test(1);
    runtime
        .request_start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_started(handle(), 2);

    let retry =
        runtime.record_endpoint_probe(endpoint(false), NativeServiceHealthProbe::default(), 3);
    assert_eq!(retry.state, NativeProcessRuntimeState::Restarting);
    assert_eq!(
        retry.last_failure,
        Some(NativeProcessRuntimeFailureKind::HealthProbeFailed)
    );

    let terminal =
        runtime.record_endpoint_probe(endpoint(false), NativeServiceHealthProbe::default(), 4);
    assert_eq!(terminal.state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        terminal.last_failure,
        Some(NativeProcessRuntimeFailureKind::HealthProbeFailed)
    );
}

#[test]
fn desktop_process_runtime_fake_process_exit_consumes_retry_budget() {
    let mut runtime = DesktopFakeProcessRuntime::enabled_for_test(1);
    runtime
        .request_start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_started(handle(), 2);

    let retry = runtime.record_process_exit(
        NativeProcessExitStatus {
            code: Some(1),
            signal: None,
        },
        3,
    );
    assert_eq!(retry.state, NativeProcessRuntimeState::Restarting);
    assert_eq!(
        retry.last_failure,
        Some(NativeProcessRuntimeFailureKind::ProcessExited)
    );

    let terminal = runtime.record_process_exit(
        NativeProcessExitStatus {
            code: Some(1),
            signal: None,
        },
        4,
    );
    assert_eq!(terminal.state, NativeProcessRuntimeState::Offline);
}

#[test]
fn desktop_process_runtime_fake_session_handoff_failure_is_fatal() {
    let mut runtime = DesktopFakeProcessRuntime::enabled_for_test(3);
    runtime
        .request_start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_started(handle(), 2);
    runtime.record_endpoint_probe(endpoint(false), healthy_probe(), 3);

    let snapshot = runtime.record_session_handoff(false, 4);

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Offline);
    assert_eq!(
        snapshot.last_failure,
        Some(NativeProcessRuntimeFailureKind::SessionHandoffFailed)
    );
}

#[test]
fn desktop_process_runtime_fake_does_not_unlock_writable_shell_without_writer_gate() {
    let mut runtime = DesktopFakeProcessRuntime::enabled_for_test(1);
    runtime
        .request_start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_started(handle(), 2);
    runtime.record_endpoint_probe(endpoint(false), healthy_probe(), 3);
    runtime.record_session_handoff(true, 4);
    runtime.mark_runtime_ready(5);

    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint(false)).expect("bind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session");

    let readiness = NativeRuntimeReadiness {
        endpoint_reachable: true,
        auth_status_valid: true,
        node_role_readable: true,
        repo_handshake_complete: true,
        writer_ready: false,
        scope_nonce_current: true,
    };

    assert!(!shell.mark_runtime_ready(readiness));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionBound);
}
