//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-service-supervisor-contract
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, NativeAdapterError,
    NativeEndpointReady, NativeProcessRuntimeSnapshot, NativeServiceFailureKind,
    NativeServiceHealthProbe, validate_native_endpoint_bases,
};
use serde_json::Value;
use thiserror::Error;

use crate::{
    DesktopBootstrap, DesktopLocalServiceEntrypointPlan, DesktopLocalServiceRuntime,
    DesktopNativeSessionCookie, DesktopProcessLauncher, DesktopProcessRuntimeError,
    DesktopSessionMaterial, DesktopShell, DesktopShellError,
};

const MAX_LOOPBACK_RESPONSE_BYTES: usize = 64 * 1024;
const LOOPBACK_HTTP_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLocalServiceProbeOutcome {
    pub endpoint: NativeEndpointReady,
    pub probe: NativeServiceHealthProbe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLocalServiceBootstrapResult {
    pub bootstrap: DesktopBootstrap,
    pub bootstrap_script: String,
    pub session_material: DesktopSessionMaterial,
    pub runtime_snapshot: NativeProcessRuntimeSnapshot,
}

pub trait DesktopLocalServiceProbe {
    fn probe_node_role(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
    ) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError>;
}

pub trait DesktopLocalServiceSessionHandoff {
    fn bind_session(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
        endpoint: &NativeEndpointReady,
    ) -> Result<DesktopSessionMaterial, DesktopLocalServiceBootstrapError>;
}

#[derive(Debug, Error)]
pub enum DesktopLocalServiceBootstrapError {
    #[error(transparent)]
    Runtime(#[from] DesktopProcessRuntimeError),
    #[error(transparent)]
    Shell(#[from] DesktopShellError),
    #[error("desktop local service health probe failed")]
    HealthProbeFailed,
    #[error("desktop local service session handoff failed")]
    SessionHandoffFailed,
    #[error("desktop local service probe URL is invalid")]
    InvalidProbeUrl,
    #[error("desktop local service endpoint is invalid")]
    InvalidEndpoint(#[from] NativeAdapterError),
    #[error("desktop local service probe HTTP status is not successful: {status}")]
    ProbeHttpStatus { status: u16 },
    #[error("desktop local service probe response is too large")]
    ProbeResponseTooLarge,
    #[error("desktop local service probe response is invalid")]
    ProbeInvalidResponse,
    #[error("desktop local service probe IO failed")]
    ProbeIo(#[source] std::io::Error),
    #[error("desktop native session bootstrap secret is missing")]
    MissingNativeSessionBootstrapSecret,
    #[error("desktop local service node-role payload is invalid")]
    InvalidNodeRolePayload,
    #[error("desktop native session cookie is invalid")]
    NativeSessionCookieInvalid,
}

pub fn run_desktop_local_service_bootstrap<L, P, H>(
    plan: &DesktopLocalServiceEntrypointPlan,
    runtime: &mut DesktopLocalServiceRuntime<L>,
    shell: &mut DesktopShell,
    probe: &mut P,
    handoff: &mut H,
    timestamp_unix_ms: i64,
) -> Result<DesktopLocalServiceBootstrapResult, DesktopLocalServiceBootstrapError>
where
    L: DesktopProcessLauncher,
    P: DesktopLocalServiceProbe,
    H: DesktopLocalServiceSessionHandoff,
{
    runtime.start(&plan.spawn_spec, timestamp_unix_ms)?;
    shell.start_service();

    let probe_outcome = match probe.probe_node_role(plan) {
        Ok(outcome) if outcome.probe.is_healthy() => outcome,
        Ok(outcome) => {
            runtime.record_endpoint_probe(
                outcome.endpoint,
                outcome.probe,
                timestamp_unix_ms.saturating_add(1),
            );
            shell.mark_supervisor_failure(
                NativeServiceFailureKind::HealthProbeFailed,
                "probe_failed",
            );
            return Err(DesktopLocalServiceBootstrapError::HealthProbeFailed);
        }
        Err(error) => {
            runtime.record_health_probe_failure(timestamp_unix_ms.saturating_add(1));
            shell.mark_supervisor_failure(
                NativeServiceFailureKind::HealthProbeFailed,
                "probe_failed",
            );
            return Err(error);
        }
    };

    let endpoint_snapshot = runtime.record_endpoint_probe(
        probe_outcome.endpoint.clone(),
        probe_outcome.probe,
        timestamp_unix_ms.saturating_add(1),
    );
    shell.bind_endpoint(probe_outcome.endpoint.clone())?;

    let session = match handoff.bind_session(plan, &probe_outcome.endpoint) {
        Ok(session) => session,
        Err(error) => {
            runtime.record_session_handoff(false, timestamp_unix_ms.saturating_add(2));
            shell.mark_supervisor_failure(
                NativeServiceFailureKind::SessionHandoffFailed,
                "session_not_bound",
            );
            return Err(error);
        }
    };
    let session_material = session.clone();
    shell.bind_session(session).map_err(|error| {
        runtime.record_session_handoff(false, timestamp_unix_ms.saturating_add(2));
        DesktopLocalServiceBootstrapError::Shell(error)
    })?;
    let runtime_snapshot =
        runtime.record_session_handoff(true, timestamp_unix_ms.saturating_add(2));
    let bootstrap = shell.bootstrap_for_web()?;
    let bootstrap_script = bootstrap.script_tag()?;

    debug_assert!(endpoint_snapshot.health_probe.is_healthy());
    Ok(DesktopLocalServiceBootstrapResult {
        bootstrap,
        bootstrap_script,
        session_material,
        runtime_snapshot,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopLoopbackHttpProbe {
    timeout: Duration,
    max_response_bytes: usize,
}

impl Default for DesktopLoopbackHttpProbe {
    fn default() -> Self {
        Self {
            timeout: LOOPBACK_HTTP_TIMEOUT,
            max_response_bytes: MAX_LOOPBACK_RESPONSE_BYTES,
        }
    }
}

impl DesktopLoopbackHttpProbe {
    pub fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            timeout,
            max_response_bytes,
        }
    }
}

impl DesktopLocalServiceProbe for DesktopLoopbackHttpProbe {
    fn probe_node_role(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
    ) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError> {
        let json = self.get_json(&format!("{}/api/node/role", plan.http_base))?;
        node_role_probe_outcome_from_json(plan, &json)
    }
}

impl DesktopLocalServiceSessionHandoff for DesktopLoopbackHttpProbe {
    fn bind_session(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
        endpoint: &NativeEndpointReady,
    ) -> Result<DesktopSessionMaterial, DesktopLocalServiceBootstrapError> {
        let Some(secret) = native_session_secret_from_plan(plan) else {
            return Err(DesktopLocalServiceBootstrapError::MissingNativeSessionBootstrapSecret);
        };
        let cookie = self.issue_native_session_cookie(plan, endpoint, secret)?;
        let json = self.get_json_with_cookie(
            &format!("{}/api/auth/status", plan.http_base),
            Some(&cookie.request_cookie_header()),
        )?;
        session_material_from_auth_status_json(&json)
            .map(|_| DesktopSessionMaterial::bound_with_native_session_cookie(cookie))
    }
}

impl DesktopLoopbackHttpProbe {
    fn get_json(&self, url: &str) -> Result<Value, DesktopLocalServiceBootstrapError> {
        self.get_json_with_cookie(url, None)
    }

    fn get_json_with_cookie(
        &self,
        url: &str,
        cookie_header: Option<&str>,
    ) -> Result<Value, DesktopLocalServiceBootstrapError> {
        let response = self.http_request("GET", url, &[], cookie_header)?;
        serde_json::from_slice(&response.body)
            .map_err(|_| DesktopLocalServiceBootstrapError::ProbeInvalidResponse)
    }

    fn issue_native_session_cookie(
        &self,
        plan: &DesktopLocalServiceEntrypointPlan,
        endpoint: &NativeEndpointReady,
        secret: &str,
    ) -> Result<DesktopNativeSessionCookie, DesktopLocalServiceBootstrapError> {
        let response = self.http_request(
            "POST",
            &format!("{}/api/auth/native-session", plan.http_base),
            &[(NATIVE_SESSION_BOOTSTRAP_HEADER, secret)],
            None,
        )?;
        let set_cookie = response
            .header("set-cookie")
            .ok_or(DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid)?;
        let domain = loopback_host_from_http_base(&endpoint.http_base)?;
        DesktopNativeSessionCookie::from_set_cookie(set_cookie, &domain)
            .map_err(|_| DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid)
    }

    fn http_request(
        &self,
        method: &'static str,
        url: &str,
        extra_headers: &[(&str, &str)],
        cookie_header: Option<&str>,
    ) -> Result<LoopbackHttpResponse, DesktopLocalServiceBootstrapError> {
        let target = parse_loopback_http_url(url)?;
        let mut stream = TcpStream::connect_timeout(&target.addr, self.timeout)
            .map_err(DesktopLocalServiceBootstrapError::ProbeIo)?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(DesktopLocalServiceBootstrapError::ProbeIo)?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(DesktopLocalServiceBootstrapError::ProbeIo)?;

        let mut request = format!(
            "{method} {} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nContent-Length: 0\r\nConnection: close\r\n",
            target.path, target.host_header
        );
        if let Some(cookie_header) = cookie_header {
            request.push_str("Cookie: ");
            request.push_str(cookie_header);
            request.push_str("\r\n");
        }
        for (name, value) in extra_headers {
            request.push_str(name);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
            .map_err(DesktopLocalServiceBootstrapError::ProbeIo)?;

        let bytes = read_capped_response(stream, self.max_response_bytes)?;
        let response = split_http_response(&bytes)?;
        let status = response.status;
        if !(200..=299).contains(&status) {
            return Err(DesktopLocalServiceBootstrapError::ProbeHttpStatus { status });
        }
        Ok(response)
    }
}

pub fn node_role_probe_outcome_from_json(
    plan: &DesktopLocalServiceEntrypointPlan,
    json: &Value,
) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError> {
    let endpoint = match json.pointer("/native_service/endpoint") {
        Some(Value::Object(endpoint)) => endpoint_from_json(endpoint)?,
        _ => NativeEndpointReady {
            http_base: plan.http_base.clone(),
            ws_base: plan.ws_base.clone(),
            node_role: json
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("native-main")
                .to_string(),
            session_bound: false,
        },
    };
    validate_native_endpoint_bases(&endpoint)?;
    Ok(DesktopLocalServiceProbeOutcome {
        probe: NativeServiceHealthProbe {
            endpoint_reachable: true,
            node_role_readable: !endpoint.node_role.trim().is_empty(),
        },
        endpoint,
    })
}

pub fn session_material_from_auth_status_json(
    json: &Value,
) -> Result<DesktopSessionMaterial, DesktopLocalServiceBootstrapError> {
    if json
        .get("authenticated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Ok(DesktopSessionMaterial::bound())
    } else {
        Err(DesktopLocalServiceBootstrapError::SessionHandoffFailed)
    }
}

fn endpoint_from_json(
    endpoint: &serde_json::Map<String, Value>,
) -> Result<NativeEndpointReady, DesktopLocalServiceBootstrapError> {
    Ok(NativeEndpointReady {
        http_base: string_field(endpoint, "http_base")?,
        ws_base: string_field(endpoint, "ws_base")?,
        node_role: string_field(endpoint, "node_role")?,
        session_bound: endpoint
            .get("session_bound")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn string_field(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<String, DesktopLocalServiceBootstrapError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(DesktopLocalServiceBootstrapError::InvalidNodeRolePayload)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopbackHttpTarget {
    addr: std::net::SocketAddr,
    host: String,
    host_header: String,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopbackHttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl LoopbackHttpResponse {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn parse_loopback_http_url(
    url: &str,
) -> Result<LoopbackHttpTarget, DesktopLocalServiceBootstrapError> {
    let Some(rest) = url.strip_prefix("http://") else {
        return Err(DesktopLocalServiceBootstrapError::InvalidProbeUrl);
    };
    let (authority, path) = rest
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((rest, "/".to_string()));
    let (host, port_text) = authority
        .rsplit_once(':')
        .ok_or(DesktopLocalServiceBootstrapError::InvalidProbeUrl)?;
    if !matches!(host, "127.0.0.1" | "localhost") {
        return Err(DesktopLocalServiceBootstrapError::InvalidProbeUrl);
    }
    let port = port_text
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(DesktopLocalServiceBootstrapError::InvalidProbeUrl)?;
    Ok(LoopbackHttpTarget {
        addr: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        host: host.to_string(),
        host_header: authority.to_string(),
        path,
    })
}

fn loopback_host_from_http_base(url: &str) -> Result<String, DesktopLocalServiceBootstrapError> {
    Ok(parse_loopback_http_url(url)?.host)
}

fn native_session_secret_from_plan(plan: &DesktopLocalServiceEntrypointPlan) -> Option<&str> {
    plan.spawn_spec
        .env
        .iter()
        .find(|binding| binding.key == NATIVE_SESSION_BOOTSTRAP_SECRET_ENV)
        .map(|binding| binding.value.as_str())
        .filter(|value| !value.trim().is_empty())
}

fn read_capped_response(
    mut stream: TcpStream,
    max_bytes: usize,
) -> Result<Vec<u8>, DesktopLocalServiceBootstrapError> {
    let mut response = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(DesktopLocalServiceBootstrapError::ProbeIo)?;
        if read == 0 {
            break;
        }
        if response.len().saturating_add(read) > max_bytes {
            return Err(DesktopLocalServiceBootstrapError::ProbeResponseTooLarge);
        }
        response.extend_from_slice(&buffer[..read]);
    }
    Ok(response)
}

fn split_http_response(
    bytes: &[u8],
) -> Result<LoopbackHttpResponse, DesktopLocalServiceBootstrapError> {
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or(DesktopLocalServiceBootstrapError::ProbeInvalidResponse)?;
    let headers = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| DesktopLocalServiceBootstrapError::ProbeInvalidResponse)?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or(DesktopLocalServiceBootstrapError::ProbeInvalidResponse)?;
    let headers = headers
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect();
    Ok(LoopbackHttpResponse {
        status,
        headers,
        body: bytes[header_end + 4..].to_vec(),
    })
}
