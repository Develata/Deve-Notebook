use deve_core::native_adapter::{
    NativeAdapterError, NativeProcessAdapterDecision, NativeProcessAdapterPolicy,
    NativeProcessBindHints, NativeProcessEnvBinding, NativeProcessExitStatus,
    NativeProcessPathResolution, NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle,
    NativeProcessRuntimeState, NativeProcessSpawnSpec, NativeRemoteTarget,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::tauri_bootstrap::desktop_local_service_error_allows_port_replan;
use crate::{
    DesktopBootstrap, DesktopCommandProcessLauncher, DesktopLocalServiceBootstrapError,
    DesktopLocalServiceRuntime, DesktopLocalServiceTauriState, DesktopNativeSessionCookie,
    DesktopProcessLauncher, DesktopProcessRuntimeError, DesktopTauriBootstrapError,
    DesktopTauriBootstrapScript, desktop_tauri_remote_browser_init_script,
    desktop_tauri_session_invalid_init_script, desktop_tauri_success_init_script,
};

fn success_bootstrap() -> DesktopBootstrap {
    DesktopBootstrap {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "native-main".to_string(),
        session_bound: true,
    }
}

fn native_session_cookie() -> DesktopNativeSessionCookie {
    DesktopNativeSessionCookie::from_set_cookie(
        "token=abc.def; Path=/; HttpOnly; SameSite=None; Secure",
        "127.0.0.1",
    )
    .expect("cookie")
}

#[test]
fn tauri_success_init_script_is_raw_js_and_session_bound() {
    let script =
        desktop_tauri_success_init_script(&success_bootstrap(), Some(native_session_cookie()))
            .expect("script");

    assert!(
        script
            .source()
            .starts_with("window.__DEVE_NATIVE_BOOTSTRAP=")
    );
    assert!(script.source().contains("\"session_bound\":true"));
    assert!(!script.source().contains("<script"));
    assert!(!script.source().contains("token"));
    assert!(!script.source().contains("secret"));
    assert!(!script.is_recovery());
    assert!(script.session_bound());
    assert!(!script.opens_authority_write_path());
    assert!(script.has_native_session_cookie());
}

#[test]
fn tauri_success_init_script_rejects_unbound_session() {
    let mut bootstrap = success_bootstrap();
    bootstrap.session_bound = false;

    assert!(matches!(
        desktop_tauri_success_init_script(&bootstrap, None),
        Err(DesktopTauriBootstrapError::SessionNotBound)
    ));
}

#[test]
fn tauri_success_init_script_requires_native_session_cookie() {
    assert!(matches!(
        desktop_tauri_success_init_script(&success_bootstrap(), None),
        Err(DesktopTauriBootstrapError::NativeSessionCookieRequired)
    ));
}

#[test]
fn tauri_success_init_script_can_carry_http_only_cookie_outside_js_source() {
    let script =
        desktop_tauri_success_init_script(&success_bootstrap(), Some(native_session_cookie()))
            .expect("script");

    assert!(script.has_native_session_cookie());
    assert!(!script.source().contains("abc.def"));
    assert!(!script.source().contains("token"));
}

#[test]
fn tauri_recovery_init_script_exposes_only_recovery_state() {
    let script = desktop_tauri_session_invalid_init_script().expect("script");

    assert!(script.is_recovery());
    assert!(!script.session_bound());
    assert!(
        script
            .source()
            .contains("\"service_state\":\"session_invalid\"")
    );
    assert!(!script.source().contains("http_base"));
    assert!(!script.source().contains("ws_base"));
    assert!(!script.source().contains("token"));
    assert!(!script.source().contains("secret"));
    assert!(!script.opens_authority_write_path());
}

#[test]
fn tauri_remote_browser_init_script_navigates_without_native_bootstrap() {
    let script = desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: "https://deve.example".to_string(),
    })
    .expect("remote script");

    assert!(script.source().contains("window.location.replace"));
    assert!(script.source().contains("https://deve.example"));
    assert!(!script.source().contains("__DEVE_NATIVE_BOOTSTRAP"));
    assert!(!script.source().contains("http_base"));
    assert!(!script.source().contains("ws_base"));
    assert!(!script.session_bound());
    assert!(!script.has_native_session_cookie());
    assert!(!script.opens_authority_write_path());
}

#[test]
fn tauri_remote_browser_init_script_rejects_non_https_origin() {
    let err = desktop_tauri_remote_browser_init_script(&NativeRemoteTarget {
        https_origin: "http://deve.example".to_string(),
    })
    .expect_err("http remote target must fail");

    assert!(matches!(
        err,
        DesktopTauriBootstrapError::RemoteTarget(NativeAdapterError::WrongScheme {
            expected_scheme: "https",
            ..
        })
    ));
}

#[test]
fn tauri_bootstrap_source_rejects_secret_bearing_material() {
    let result = DesktopTauriBootstrapScript::new(
        "window.__DEVE_NATIVE_BOOTSTRAP={token:\"x\"};".to_string(),
        false,
        true,
        None,
    );

    assert!(matches!(
        result,
        Err(DesktopTauriBootstrapError::ForbiddenMaterial { marker: "token" })
    ));
}

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
