use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NativeEndpointReady, validate_native_endpoint_bases,
};
use serde_json::Value;

use super::cookie::MobileNativeSessionCookie;
use super::{MobileEmbeddedBackendError, MobileEmbeddedBackendPlan};

const MAX_LOOPBACK_RESPONSE_BYTES: usize = 64 * 1024;
const LOOPBACK_HTTP_TIMEOUT: Duration = Duration::from_secs(1);
const LOOPBACK_HTTP_STARTUP_GRACE: Duration = Duration::from_secs(15);
const LOOPBACK_HTTP_RETRY_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MobileLoopbackHttpProbe {
    timeout: Duration,
    max_response_bytes: usize,
    startup_grace: Duration,
    retry_interval: Duration,
}

impl Default for MobileLoopbackHttpProbe {
    fn default() -> Self {
        Self {
            timeout: LOOPBACK_HTTP_TIMEOUT,
            max_response_bytes: MAX_LOOPBACK_RESPONSE_BYTES,
            startup_grace: LOOPBACK_HTTP_STARTUP_GRACE,
            retry_interval: LOOPBACK_HTTP_RETRY_INTERVAL,
        }
    }
}

impl MobileLoopbackHttpProbe {
    pub(super) fn probe_node_role(
        &self,
        plan: &MobileEmbeddedBackendPlan,
    ) -> Result<NativeEndpointReady, MobileEmbeddedBackendError> {
        let json =
            self.get_json_with_startup_retry(&format!("{}/api/node/role", plan.http_base))?;
        endpoint_from_node_role_json(plan, &json)
    }

    pub(super) fn bind_native_session(
        &self,
        plan: &MobileEmbeddedBackendPlan,
        endpoint: &NativeEndpointReady,
        secret: &str,
    ) -> Result<MobileNativeSessionCookie, MobileEmbeddedBackendError> {
        let cookie = self.issue_native_session_cookie(plan, endpoint, secret)?;
        let json = self.get_json_with_cookie(
            &format!("{}/api/auth/status", plan.http_base),
            Some(&cookie.request_cookie_header()),
        )?;
        if json
            .get("authenticated")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            Ok(cookie)
        } else {
            Err(MobileEmbeddedBackendError::NativeSessionHandoffFailed)
        }
    }

    fn get_json_with_startup_retry(&self, url: &str) -> Result<Value, MobileEmbeddedBackendError> {
        let deadline = Instant::now() + self.startup_grace;
        loop {
            match self.get_json(url) {
                Ok(json) => return Ok(json),
                Err(error) if is_retryable_startup_probe_error(&error) => {
                    let now = Instant::now();
                    if now >= deadline || self.retry_interval.is_zero() {
                        return Err(error);
                    }
                    std::thread::sleep(self.retry_interval.min(deadline.duration_since(now)));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn get_json(&self, url: &str) -> Result<Value, MobileEmbeddedBackendError> {
        self.get_json_with_cookie(url, None)
    }

    fn get_json_with_cookie(
        &self,
        url: &str,
        cookie_header: Option<&str>,
    ) -> Result<Value, MobileEmbeddedBackendError> {
        let response = self.http_request("GET", url, &[], cookie_header)?;
        serde_json::from_slice(&response.body)
            .map_err(|_| MobileEmbeddedBackendError::ProbeInvalidResponse)
    }

    fn issue_native_session_cookie(
        &self,
        plan: &MobileEmbeddedBackendPlan,
        endpoint: &NativeEndpointReady,
        secret: &str,
    ) -> Result<MobileNativeSessionCookie, MobileEmbeddedBackendError> {
        let response = self.http_request(
            "POST",
            &format!("{}/api/auth/native-session", plan.http_base),
            &[(NATIVE_SESSION_BOOTSTRAP_HEADER, secret)],
            None,
        )?;
        let set_cookie = response
            .header("set-cookie")
            .ok_or(MobileEmbeddedBackendError::NativeSessionCookieInvalid)?;
        let domain = loopback_host_from_http_base(&endpoint.http_base)?;
        MobileNativeSessionCookie::from_set_cookie(set_cookie, &domain)
    }

    fn http_request(
        &self,
        method: &'static str,
        url: &str,
        extra_headers: &[(&str, &str)],
        cookie_header: Option<&str>,
    ) -> Result<LoopbackHttpResponse, MobileEmbeddedBackendError> {
        let target = parse_loopback_http_url(url)?;
        let mut stream = TcpStream::connect_timeout(&target.addr, self.timeout)
            .map_err(MobileEmbeddedBackendError::ProbeIo)?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(MobileEmbeddedBackendError::ProbeIo)?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(MobileEmbeddedBackendError::ProbeIo)?;

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
            .map_err(MobileEmbeddedBackendError::ProbeIo)?;

        let bytes = read_capped_response(stream, self.max_response_bytes)?;
        let response = split_http_response(&bytes)?;
        let status = response.status;
        if !(200..=299).contains(&status) {
            return Err(MobileEmbeddedBackendError::ProbeHttpStatus { status });
        }
        Ok(response)
    }
}

fn endpoint_from_node_role_json(
    plan: &MobileEmbeddedBackendPlan,
    json: &Value,
) -> Result<NativeEndpointReady, MobileEmbeddedBackendError> {
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
    Ok(endpoint)
}

fn endpoint_from_json(
    endpoint: &serde_json::Map<String, Value>,
) -> Result<NativeEndpointReady, MobileEmbeddedBackendError> {
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
) -> Result<String, MobileEmbeddedBackendError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(MobileEmbeddedBackendError::ProbeInvalidResponse)
}

fn is_retryable_startup_probe_error(error: &MobileEmbeddedBackendError) -> bool {
    matches!(
        error,
        MobileEmbeddedBackendError::ProbeIo(source)
            if matches!(
                source.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::TimedOut
            )
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopbackHttpTarget {
    addr: SocketAddr,
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

fn parse_loopback_http_url(url: &str) -> Result<LoopbackHttpTarget, MobileEmbeddedBackendError> {
    let Some(rest) = url.strip_prefix("http://") else {
        return Err(MobileEmbeddedBackendError::InvalidProbeUrl);
    };
    let (authority, path) = rest
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((rest, "/".to_string()));
    let (host, port_text) = authority
        .rsplit_once(':')
        .ok_or(MobileEmbeddedBackendError::InvalidProbeUrl)?;
    if !matches!(host, "127.0.0.1" | "localhost") {
        return Err(MobileEmbeddedBackendError::InvalidProbeUrl);
    }
    let port = port_text
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(MobileEmbeddedBackendError::InvalidProbeUrl)?;
    Ok(LoopbackHttpTarget {
        addr: SocketAddr::from(([127, 0, 0, 1], port)),
        host: host.to_string(),
        host_header: authority.to_string(),
        path,
    })
}

fn loopback_host_from_http_base(url: &str) -> Result<String, MobileEmbeddedBackendError> {
    Ok(parse_loopback_http_url(url)?.host)
}

fn read_capped_response(
    mut stream: TcpStream,
    max_bytes: usize,
) -> Result<Vec<u8>, MobileEmbeddedBackendError> {
    let mut response = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(MobileEmbeddedBackendError::ProbeIo)?;
        if read == 0 {
            break;
        }
        if response.len().saturating_add(read) > max_bytes {
            return Err(MobileEmbeddedBackendError::ProbeResponseTooLarge);
        }
        response.extend_from_slice(&buffer[..read]);
    }
    Ok(response)
}

fn split_http_response(bytes: &[u8]) -> Result<LoopbackHttpResponse, MobileEmbeddedBackendError> {
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or(MobileEmbeddedBackendError::ProbeInvalidResponse)?;
    let headers = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| MobileEmbeddedBackendError::ProbeInvalidResponse)?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or(MobileEmbeddedBackendError::ProbeInvalidResponse)?;
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
