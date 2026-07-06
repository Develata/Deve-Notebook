use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use deve_core::native_adapter::{
    NativeProcessAdapterDecision, NativeProcessAdapterPolicy, NativeProcessBindHints,
    NativeProcessEnvBinding, NativeProcessExitStatus, NativeProcessPathResolution,
    NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle, NativeProcessRuntimeState,
    NativeProcessSpawnSpec,
};

use crate::tauri_bootstrap::desktop_local_service_error_allows_port_replan;
use crate::{
    DesktopCommandProcessLauncher, DesktopLocalServiceBootstrapError, DesktopLocalServiceRuntime,
    DesktopLocalServiceTauriState, DesktopProcessLauncher, DesktopProcessRuntimeError,
};

#[test]
fn tauri_local_service_replans_port_only_for_retryable_startup_failures() {
    assert!(desktop_local_service_error_allows_port_replan(
        &DesktopLocalServiceBootstrapError::Runtime(DesktopProcessRuntimeError::SpawnFailed {
            kind: NativeProcessRuntimeFailureKind::BindFailed,
            source: std::io::Error::new(std::io::ErrorKind::AddrInUse, "port occupied"),
        })
    ));
    assert!(desktop_local_service_error_allows_port_replan(
        &DesktopLocalServiceBootstrapError::ProbeIo(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "service not reachable",
        ))
    ));
    assert!(desktop_local_service_error_allows_port_replan(
        &DesktopLocalServiceBootstrapError::HealthProbeFailed
    ));
    assert!(!desktop_local_service_error_allows_port_replan(
        &DesktopLocalServiceBootstrapError::SessionHandoffFailed
    ));
    assert!(!desktop_local_service_error_allows_port_replan(
        &DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid
    ));
}

#[test]
fn tauri_state_keeps_successful_runtime_observable() {
    let runtime = DesktopLocalServiceRuntime::with_launcher(
        NativeProcessAdapterPolicy {
            decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
            child_process_runtime_enabled: true,
            embedded_service_runtime_enabled: false,
            packaging_gate_required: true,
            authority_writes_allowed: false,
        },
        1,
        DesktopCommandProcessLauncher::default(),
    );
    let state = DesktopLocalServiceTauriState::new(runtime);
    let snapshot = state.runtime_snapshot().expect("snapshot");

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Disabled);
    assert!(snapshot.child_process_runtime_enabled);
    assert!(!snapshot.authority_writes_allowed);
}

#[test]
fn tauri_state_drop_stops_running_local_service() {
    let stop_count = Arc::new(AtomicUsize::new(0));
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(
        NativeProcessAdapterPolicy {
            decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
            child_process_runtime_enabled: true,
            embedded_service_runtime_enabled: false,
            packaging_gate_required: true,
            authority_writes_allowed: false,
        },
        1,
        CountingLauncher {
            stop_count: Arc::clone(&stop_count),
        },
    );
    runtime
        .start(&valid_spawn_spec(), 1)
        .expect("start fake local service");

    let state = DesktopLocalServiceTauriState::new(runtime);
    drop(state);

    assert_eq!(stop_count.load(Ordering::SeqCst), 1);
}

#[derive(Debug)]
struct CountingLauncher {
    stop_count: Arc<AtomicUsize>,
}

impl DesktopProcessLauncher for CountingLauncher {
    fn spawn_service(
        &mut self,
        _spec: &NativeProcessSpawnSpec,
    ) -> Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError> {
        Ok(NativeProcessRuntimeHandle {
            handle_id: "fake-child".to_string(),
            platform_pid: Some(42),
        })
    }

    fn stop_service(
        &mut self,
    ) -> Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError> {
        self.stop_count.fetch_add(1, Ordering::SeqCst);
        Ok(Some(NativeProcessExitStatus {
            code: Some(0),
            signal: None,
        }))
    }
}

fn valid_spawn_spec() -> NativeProcessSpawnSpec {
    let root = std::env::current_dir().expect("current dir");
    NativeProcessSpawnSpec {
        executable: root.join(if cfg!(windows) {
            "target/native/deve_cli.exe"
        } else {
            "target/native/deve_cli"
        }),
        argv: vec![
            "serve".to_string(),
            "--native-loopback".to_string(),
            "--port".to_string(),
            "3001".to_string(),
        ],
        cwd: root.clone(),
        env_allowlist: vec!["DEVE_PROFILE".to_string()],
        env: vec![NativeProcessEnvBinding {
            key: "DEVE_PROFILE".to_string(),
            value: "standard".to_string(),
        }],
        profile: "standard".to_string(),
        config_path: root.join("config.toml"),
        ledger_path: root.join("ledger"),
        bind_hints: NativeProcessBindHints {
            http_host: "127.0.0.1".to_string(),
            http_port: Some(3001),
            ws_host: "127.0.0.1".to_string(),
            ws_port: Some(3001),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    }
}
