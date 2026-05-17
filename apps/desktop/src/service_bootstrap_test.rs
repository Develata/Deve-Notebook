use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use deve_core::config::AppProfile;
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, NativeEndpointReady,
    NativeProcessAdapterDecision, NativeProcessAdapterPolicy, NativeProcessExitStatus,
    NativeProcessRuntimeFailureKind, NativeProcessRuntimeHandle, NativeProcessRuntimeState,
    NativeProcessSpawnSpec, NativeServiceHealthProbe,
};
use serde_json::json;

use crate::{
    DesktopLocalServiceBootstrapError, DesktopLocalServiceEntrypointInput,
    DesktopLocalServiceEntrypointPolicy, DesktopLocalServiceProbe, DesktopLocalServiceProbeOutcome,
    DesktopLocalServiceRuntime, DesktopLocalServiceSessionHandoff, DesktopLoopbackHttpProbe,
    DesktopProcessLauncher, DesktopProcessRuntimeError, DesktopSessionMaterial, DesktopShell,
    node_role_probe_outcome_from_json, plan_desktop_local_service_entrypoint,
    run_desktop_local_service_bootstrap, session_material_from_auth_status_json,
};

fn abs(path: &str) -> PathBuf {
    std::env::current_dir().expect("current dir").join(path)
}

fn plan() -> crate::DesktopLocalServiceEntrypointPlan {
    plan_desktop_local_service_entrypoint(
        DesktopLocalServiceEntrypointPolicy::opt_in_enabled(),
        DesktopLocalServiceEntrypointInput {
            current_exe: abs("target/debug/deve_desktop"),
            data_root: abs("desktop-data"),
            port: 39101,
            profile: AppProfile::Standard,
        },
    )
    .expect("plan")
    .expect("enabled")
}

fn endpoint(session_bound: bool) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:39101".to_string(),
        ws_base: "ws://127.0.0.1:39101".to_string(),
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

#[derive(Debug, Default)]
struct FakeLauncher {
    calls: usize,
}

impl DesktopProcessLauncher for FakeLauncher {
    fn spawn_service(
        &mut self,
        _spec: &NativeProcessSpawnSpec,
    ) -> Result<NativeProcessRuntimeHandle, DesktopProcessRuntimeError> {
        self.calls += 1;
        Ok(NativeProcessRuntimeHandle {
            handle_id: "fake-child".to_string(),
            platform_pid: Some(42),
        })
    }

    fn stop_service(
        &mut self,
    ) -> Result<Option<NativeProcessExitStatus>, DesktopProcessRuntimeError> {
        Ok(None)
    }
}

#[derive(Debug)]
struct FakeProbe {
    outcome: DesktopLocalServiceProbeOutcome,
}

impl DesktopLocalServiceProbe for FakeProbe {
    fn probe_node_role(
        &mut self,
        _plan: &crate::DesktopLocalServiceEntrypointPlan,
    ) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError> {
        Ok(self.outcome.clone())
    }
}

#[derive(Debug)]
struct FakeSessionHandoff {
    session_bound: bool,
}

impl DesktopLocalServiceSessionHandoff for FakeSessionHandoff {
    fn bind_session(
        &mut self,
        _plan: &crate::DesktopLocalServiceEntrypointPlan,
        _endpoint: &NativeEndpointReady,
    ) -> Result<DesktopSessionMaterial, DesktopLocalServiceBootstrapError> {
        if self.session_bound {
            Ok(DesktopSessionMaterial::bound())
        } else {
            Err(DesktopLocalServiceBootstrapError::SessionHandoffFailed)
        }
    }
}

fn enabled_policy() -> NativeProcessAdapterPolicy {
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
        child_process_runtime_enabled: true,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    }
}

#[test]
fn desktop_local_service_bootstrap_requires_probe_and_session_before_script() {
    let plan = plan();
    let mut runtime =
        DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, FakeLauncher::default());
    let mut shell = DesktopShell::new();
    let mut probe = FakeProbe {
        outcome: DesktopLocalServiceProbeOutcome {
            endpoint: endpoint(false),
            probe: healthy_probe(),
        },
    };
    let mut handoff = FakeSessionHandoff {
        session_bound: true,
    };

    let result = run_desktop_local_service_bootstrap(
        &plan,
        &mut runtime,
        &mut shell,
        &mut probe,
        &mut handoff,
        10,
    )
    .expect("bootstrap");

    assert_eq!(result.bootstrap.http_base, "http://127.0.0.1:39101");
    assert_eq!(result.bootstrap.ws_base, "ws://127.0.0.1:39101");
    assert!(result.bootstrap.session_bound);
    assert!(
        result
            .bootstrap_script
            .contains("window.__DEVE_NATIVE_BOOTSTRAP")
    );
    assert_eq!(
        result.runtime_snapshot.state,
        NativeProcessRuntimeState::SessionHandoffReady
    );
    assert!(!result.runtime_snapshot.authority_writes_allowed);
    assert!(shell.snapshot().endpoint.expect("endpoint").session_bound);
}

#[test]
fn desktop_local_service_bootstrap_blocks_unhealthy_probe() {
    let plan = plan();
    let mut runtime =
        DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, FakeLauncher::default());
    let mut shell = DesktopShell::new();
    let mut probe = FakeProbe {
        outcome: DesktopLocalServiceProbeOutcome {
            endpoint: endpoint(false),
            probe: NativeServiceHealthProbe::default(),
        },
    };
    let mut handoff = FakeSessionHandoff {
        session_bound: true,
    };

    let error = run_desktop_local_service_bootstrap(
        &plan,
        &mut runtime,
        &mut shell,
        &mut probe,
        &mut handoff,
        10,
    )
    .expect_err("probe failure blocks bootstrap");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::HealthProbeFailed
    ));
    assert!(shell.recovery_bootstrap_for_web().is_some());
    assert!(!runtime.snapshot().authority_writes_allowed);
}

#[test]
fn desktop_local_service_bootstrap_blocks_session_handoff_failure() {
    let plan = plan();
    let mut runtime =
        DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, FakeLauncher::default());
    let mut shell = DesktopShell::new();
    let mut probe = FakeProbe {
        outcome: DesktopLocalServiceProbeOutcome {
            endpoint: endpoint(false),
            probe: healthy_probe(),
        },
    };
    let mut handoff = FakeSessionHandoff {
        session_bound: false,
    };

    let error = run_desktop_local_service_bootstrap(
        &plan,
        &mut runtime,
        &mut shell,
        &mut probe,
        &mut handoff,
        10,
    )
    .expect_err("session failure blocks bootstrap");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::SessionHandoffFailed
    ));
    assert_eq!(
        runtime.snapshot().last_failure,
        Some(NativeProcessRuntimeFailureKind::SessionHandoffFailed)
    );
    assert!(shell.recovery_bootstrap_for_web().is_some());
}

#[test]
fn desktop_node_role_payload_maps_native_endpoint() {
    let plan = plan();
    let outcome = node_role_probe_outcome_from_json(
        &plan,
        &json!({
            "role": "native-main",
            "native_service": {
                "state": "session_pending",
                "endpoint": {
                    "http_base": "http://127.0.0.1:39101",
                    "ws_base": "ws://127.0.0.1:39101",
                    "node_role": "native-main",
                    "session_bound": false
                }
            }
        }),
    )
    .expect("node role payload");

    assert!(outcome.probe.is_healthy());
    assert_eq!(outcome.endpoint.node_role, "native-main");
    assert!(!outcome.endpoint.session_bound);
}

#[test]
fn desktop_auth_status_controls_session_material() {
    assert!(session_material_from_auth_status_json(&json!({"authenticated": true})).is_ok());
    assert!(matches!(
        session_material_from_auth_status_json(&json!({"authenticated": false})),
        Err(DesktopLocalServiceBootstrapError::SessionHandoffFailed)
    ));
}

#[test]
fn desktop_loopback_http_probe_reads_node_role() {
    let node_role_base = spawn_json_response(json!({
        "role": "native-main",
        "native_service": {
            "state": "session_pending",
            "endpoint": {
                "http_base": "http://127.0.0.1:39101",
                "ws_base": "ws://127.0.0.1:39101",
                "node_role": "native-main",
                "session_bound": false
            }
        }
    }));
    let mut plan = plan();
    plan.http_base = node_role_base;
    let mut probe = DesktopLoopbackHttpProbe::default();

    let outcome = probe.probe_node_role(&plan).expect("node role probe");
    assert!(outcome.probe.is_healthy());
    assert_eq!(outcome.endpoint.node_role, "native-main");
}

#[test]
fn desktop_loopback_http_probe_requires_native_session_secret() {
    let mut plan = plan();
    plan.spawn_spec
        .env
        .retain(|binding| binding.key != NATIVE_SESSION_BOOTSTRAP_SECRET_ENV);
    let endpoint = endpoint(false);
    let mut probe = DesktopLoopbackHttpProbe::default();

    let error = probe
        .bind_session(&plan, &endpoint)
        .expect_err("missing native session secret fails closed");

    assert!(matches!(
        error,
        DesktopLocalServiceBootstrapError::MissingNativeSessionBootstrapSecret
    ));
}

#[test]
fn desktop_loopback_http_probe_issues_native_session_cookie_before_auth_status() {
    let mut plan = plan();
    plan.http_base = spawn_native_session_then_auth_status();
    let endpoint = endpoint(false);
    let mut probe = DesktopLoopbackHttpProbe::default();

    let session = probe
        .bind_session(&plan, &endpoint)
        .expect("native session");
    let cookie = session.native_session_cookie().expect("native cookie");

    assert_eq!(cookie.name(), "token");
    assert_eq!(cookie.domain(), "127.0.0.1");
    assert_eq!(cookie.path(), "/");
    assert!(cookie.http_only());
    assert_eq!(cookie.same_site(), "Strict");
    assert!(!format!("{:?}", cookie).contains("native.jwt"));
}

#[test]
fn desktop_native_session_cookie_rejects_non_loopback_domain() {
    let error = crate::DesktopNativeSessionCookie::from_set_cookie(
        "token=native.jwt; Path=/; HttpOnly; SameSite=Strict",
        "example.com",
    )
    .expect_err("non-loopback domain rejected");

    assert!(matches!(
        error,
        crate::DesktopShellError::NativeSessionCookieInvalid
    ));
}

fn spawn_json_response(body: serde_json::Value) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request = [0u8; 1024];
        let _ = stream.read(&mut request);
        let body = body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    format!("http://{}", addr)
}

fn spawn_native_session_then_auth_status() -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");
    thread::spawn(move || {
        for idx in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = [0u8; 2048];
            let read = stream.read(&mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..read]);
            if idx == 0 {
                assert!(request.starts_with("POST /api/auth/native-session "));
                assert!(request.contains(NATIVE_SESSION_BOOTSTRAP_HEADER));
                let body = LoginSuccess::json();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nSet-Cookie: token=native.jwt; Path=/; HttpOnly; SameSite=Strict\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            } else {
                assert!(request.starts_with("GET /api/auth/status "));
                assert!(request.contains("Cookie: token=native.jwt"));
                let body = json!({"authenticated": true}).to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
        }
    });
    format!("http://{}", addr)
}

struct LoginSuccess;

impl LoginSuccess {
    fn json() -> String {
        json!({"success": true}).to_string()
    }
}
