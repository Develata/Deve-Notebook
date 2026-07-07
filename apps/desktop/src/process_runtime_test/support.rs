use deve_core::native_adapter::{
    NativeEndpointReady, NativeProcessAdapterDecision, NativeProcessAdapterPolicy,
    NativeProcessBindHints, NativeProcessEnvBinding, NativeProcessExitStatus,
    NativeProcessPathResolution, NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle,
    NativeProcessSpawnSpec, NativeServiceHealthProbe,
};

use crate::process_runtime::{DesktopProcessLauncher, DesktopProcessRuntimeError};

pub(super) fn valid_spawn_spec() -> NativeProcessSpawnSpec {
    let root = std::env::current_dir().expect("current dir");
    NativeProcessSpawnSpec {
        executable: root.join("target/native/deve_cli"),
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
            ws_host: "localhost".to_string(),
            ws_port: Some(3001),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    }
}

#[cfg(windows)]
pub(crate) fn windows_cmd_ping_spawn_spec() -> NativeProcessSpawnSpec {
    let root = std::env::current_dir().expect("current dir");
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    NativeProcessSpawnSpec {
        executable: std::path::Path::new("C:\\Windows\\System32\\cmd.exe").to_path_buf(),
        argv: vec!["/C".to_string(), "ping -n 60 127.0.0.1 >NUL".to_string()],
        cwd: root.clone(),
        env_allowlist: vec!["SystemRoot".to_string()],
        env: vec![NativeProcessEnvBinding {
            key: "SystemRoot".to_string(),
            value: system_root,
        }],
        profile: "test".to_string(),
        config_path: root.join("config.toml"),
        ledger_path: root.join("ledger"),
        bind_hints: NativeProcessBindHints {
            http_host: "127.0.0.1".to_string(),
            http_port: Some(1),
            ws_host: "127.0.0.1".to_string(),
            ws_port: Some(1),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    }
}

pub(super) fn handle() -> NativeProcessRuntimeHandle {
    NativeProcessRuntimeHandle {
        handle_id: "fake-child-1".to_string(),
        platform_pid: Some(4242),
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingLauncher {
    calls: usize,
    result: Option<Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError>>,
    stop_calls: usize,
    stop_result: Option<Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError>>,
}

impl RecordingLauncher {
    pub(super) fn with_handle(handle: NativeProcessRuntimeHandle) -> Self {
        Self {
            calls: 0,
            result: Some(Ok(handle)),
            stop_calls: 0,
            stop_result: Some(Ok(Some(NativeProcessExitStatus {
                code: Some(0),
                signal: None,
            }))),
        }
    }

    pub(super) fn with_error(error: DesktopProcessRuntimeError) -> Self {
        Self {
            calls: 0,
            result: Some(Err(error)),
            stop_calls: 0,
            stop_result: Some(Ok(None)),
        }
    }

    pub(super) fn with_handle_and_stop_error(handle: NativeProcessRuntimeHandle) -> Self {
        Self {
            calls: 0,
            result: Some(Ok(handle)),
            stop_calls: 0,
            stop_result: Some(Err(DesktopProcessRuntimeError::StopFailed {
                source: std::io::Error::other("fake stop failure"),
            })),
        }
    }
}

impl DesktopProcessLauncher for RecordingLauncher {
    fn spawn_service(
        &mut self,
        _spec: &NativeProcessSpawnSpec,
    ) -> Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError> {
        self.calls += 1;
        self.result.take().unwrap_or_else(|| {
            Ok(NativeProcessRuntimeHandle {
                handle_id: format!("fake-child-{}", self.calls),
                platform_pid: Some(self.calls as u32),
            })
        })
    }

    fn stop_service(
        &mut self,
    ) -> Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError> {
        self.stop_calls += 1;
        self.stop_result.take().unwrap_or(Ok(None))
    }
}

pub(super) fn enabled_policy() -> NativeProcessAdapterPolicy {
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
        child_process_runtime_enabled: true,
        embedded_service_runtime_enabled: false,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    }
}

pub(super) fn closed_policy() -> NativeProcessAdapterPolicy {
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
        child_process_runtime_enabled: false,
        embedded_service_runtime_enabled: false,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    }
}

pub(super) fn endpoint(session_bound: bool) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound,
    }
}

pub(super) fn healthy_probe() -> NativeServiceHealthProbe {
    NativeServiceHealthProbe {
        endpoint_reachable: true,
        node_role_readable: true,
    }
}

pub(super) fn spawn_missing_error() -> DesktopProcessRuntimeError {
    DesktopProcessRuntimeError::SpawnFailed {
        kind: NativeProcessRuntimeFailureKind::SpawnExecutableMissing,
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
    }
}

pub(super) fn containment_error() -> DesktopProcessRuntimeError {
    DesktopProcessRuntimeError::ContainmentFailed {
        source: std::io::Error::other("child process containment failed"),
    }
}
