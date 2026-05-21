use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use deve_core::config::AppProfile;
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NativeEndpointReady, NativeProcessAdapterDecision,
    NativeProcessAdapterPolicy, NativeProcessExitStatus, NativeProcessRuntimeHandle,
    NativeProcessSpawnSpec, NativeServiceHealthProbe,
};
use serde_json::json;

use crate::{
    DesktopLocalServiceBootstrapError, DesktopLocalServiceEntrypointInput,
    DesktopLocalServiceEntrypointPlan, DesktopLocalServiceEntrypointPolicy,
    DesktopLocalServiceProbe, DesktopLocalServiceProbeOutcome, DesktopLocalServiceSessionHandoff,
    DesktopProcessLauncher, DesktopProcessRuntimeError, DesktopSessionMaterial,
    plan_desktop_local_service_entrypoint,
};

fn abs(path: &str) -> PathBuf {
    std::env::current_dir().expect("current dir").join(path)
}

pub(super) fn plan() -> DesktopLocalServiceEntrypointPlan {
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

pub(super) fn endpoint(session_bound: bool) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:39101".to_string(),
        ws_base: "ws://127.0.0.1:39101".to_string(),
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

#[derive(Debug, Default)]
pub(super) struct FakeLauncher {
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
pub(super) struct FakeProbe {
    pub(super) outcome: DesktopLocalServiceProbeOutcome,
}

impl DesktopLocalServiceProbe for FakeProbe {
    fn probe_node_role(
        &mut self,
        _plan: &DesktopLocalServiceEntrypointPlan,
    ) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError> {
        Ok(self.outcome.clone())
    }
}

#[derive(Debug)]
pub(super) struct FakeSessionHandoff {
    pub(super) session_bound: bool,
}

impl DesktopLocalServiceSessionHandoff for FakeSessionHandoff {
    fn bind_session(
        &mut self,
        _plan: &DesktopLocalServiceEntrypointPlan,
        _endpoint: &NativeEndpointReady,
    ) -> Result<DesktopSessionMaterial, DesktopLocalServiceBootstrapError> {
        if self.session_bound {
            Ok(DesktopSessionMaterial::bound())
        } else {
            Err(DesktopLocalServiceBootstrapError::SessionHandoffFailed)
        }
    }
}

pub(super) fn enabled_policy() -> NativeProcessAdapterPolicy {
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
        child_process_runtime_enabled: true,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    }
}

pub(super) fn spawn_json_response(body: serde_json::Value) -> String {
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

pub(super) fn spawn_delayed_json_response(delay: Duration, body: serde_json::Value) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let addr = listener.local_addr().expect("listener addr");
    drop(listener);
    thread::spawn(move || {
        thread::sleep(delay);
        let listener = TcpListener::bind(addr).expect("bind delayed test listener");
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

pub(super) fn spawn_native_session_then_auth_status() -> String {
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
