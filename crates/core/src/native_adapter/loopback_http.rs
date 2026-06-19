//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!
//! Shared loopback HTTP probe infrastructure for native LocalBackend shells.

use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use serde_json::Value;
use thiserror::Error;

pub const DEFAULT_MAX_LOOPBACK_RESPONSE_BYTES: usize = 64 * 1024;
pub const DEFAULT_LOOPBACK_HTTP_TIMEOUT: Duration = Duration::from_secs(1);
pub const DEFAULT_LOOPBACK_HTTP_STARTUP_GRACE: Duration = Duration::from_secs(15);
pub const DEFAULT_LOOPBACK_HTTP_RETRY_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLoopbackHttpTarget {
    pub addr: SocketAddr,
    pub host: String,
    pub host_header: String,
    pub path: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct NativeLoopbackHttpResponse {
    pub status: u16,
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl fmt::Debug for NativeLoopbackHttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeLoopbackHttpResponse")
            .field("status", &self.status)
            .field("headers", &redacted_headers(&self.headers))
            .field("body_len", &self.body.len())
            .finish()
    }
}

impl NativeLoopbackHttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Error)]
pub enum NativeLoopbackHttpError {
    #[error("native loopback HTTP URL is invalid")]
    InvalidUrl,
    #[error("native loopback HTTP status is not successful: {status}")]
    HttpStatus { status: u16 },
    #[error("native loopback HTTP response is too large")]
    ResponseTooLarge,
    #[error("native loopback HTTP response is invalid")]
    InvalidResponse,
    #[error("native loopback HTTP IO failed")]
    Io(#[source] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeLoopbackHttpProbe {
    timeout: Duration,
    max_response_bytes: usize,
    startup_grace: Duration,
    retry_interval: Duration,
}

impl Default for NativeLoopbackHttpProbe {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_LOOPBACK_HTTP_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_LOOPBACK_RESPONSE_BYTES,
            startup_grace: DEFAULT_LOOPBACK_HTTP_STARTUP_GRACE,
            retry_interval: DEFAULT_LOOPBACK_HTTP_RETRY_INTERVAL,
        }
    }
}

impl NativeLoopbackHttpProbe {
    pub fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            timeout,
            max_response_bytes,
            startup_grace: DEFAULT_LOOPBACK_HTTP_STARTUP_GRACE,
            retry_interval: DEFAULT_LOOPBACK_HTTP_RETRY_INTERVAL,
        }
    }

    pub fn with_startup_retry(mut self, startup_grace: Duration, retry_interval: Duration) -> Self {
        self.startup_grace = startup_grace;
        self.retry_interval = retry_interval;
        self
    }

    pub fn get_json_with_startup_retry(&self, url: &str) -> Result<Value, NativeLoopbackHttpError> {
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

    pub fn get_json(&self, url: &str) -> Result<Value, NativeLoopbackHttpError> {
        self.get_json_with_cookie(url, None)
    }

    pub fn get_json_with_cookie(
        &self,
        url: &str,
        cookie_header: Option<&str>,
    ) -> Result<Value, NativeLoopbackHttpError> {
        let response = self.http_request("GET", url, &[], cookie_header)?;
        serde_json::from_slice(&response.body).map_err(|_| NativeLoopbackHttpError::InvalidResponse)
    }

    pub fn http_request(
        &self,
        method: &'static str,
        url: &str,
        extra_headers: &[(&str, &str)],
        cookie_header: Option<&str>,
    ) -> Result<NativeLoopbackHttpResponse, NativeLoopbackHttpError> {
        let target = parse_loopback_http_url(url)?;
        let mut stream = TcpStream::connect_timeout(&target.addr, self.timeout)
            .map_err(NativeLoopbackHttpError::Io)?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(NativeLoopbackHttpError::Io)?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(NativeLoopbackHttpError::Io)?;

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
            .map_err(NativeLoopbackHttpError::Io)?;

        let bytes = read_capped_response(stream, self.max_response_bytes)?;
        let response = split_http_response(&bytes)?;
        if !(200..=299).contains(&response.status) {
            return Err(NativeLoopbackHttpError::HttpStatus {
                status: response.status,
            });
        }
        Ok(response)
    }
}

pub fn is_retryable_startup_probe_error(error: &NativeLoopbackHttpError) -> bool {
    matches!(
        error,
        NativeLoopbackHttpError::Io(source)
            if matches!(
                source.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::TimedOut
            )
    )
}

pub fn parse_loopback_http_url(
    url: &str,
) -> Result<NativeLoopbackHttpTarget, NativeLoopbackHttpError> {
    let Some(rest) = url.strip_prefix("http://") else {
        return Err(NativeLoopbackHttpError::InvalidUrl);
    };
    let (authority, path) = rest
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((rest, "/".to_string()));
    let (host, port_text) = authority
        .rsplit_once(':')
        .ok_or(NativeLoopbackHttpError::InvalidUrl)?;
    if !matches!(host, "127.0.0.1" | "localhost") {
        return Err(NativeLoopbackHttpError::InvalidUrl);
    }
    let port = port_text
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(NativeLoopbackHttpError::InvalidUrl)?;
    Ok(NativeLoopbackHttpTarget {
        addr: SocketAddr::from(([127, 0, 0, 1], port)),
        host: host.to_string(),
        host_header: authority.to_string(),
        path,
    })
}

pub fn loopback_host_from_http_base(url: &str) -> Result<String, NativeLoopbackHttpError> {
    Ok(parse_loopback_http_url(url)?.host)
}

fn read_capped_response(
    mut stream: TcpStream,
    max_bytes: usize,
) -> Result<Vec<u8>, NativeLoopbackHttpError> {
    let mut response = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(NativeLoopbackHttpError::Io)?;
        if read == 0 {
            break;
        }
        if response.len().saturating_add(read) > max_bytes {
            return Err(NativeLoopbackHttpError::ResponseTooLarge);
        }
        response.extend_from_slice(&buffer[..read]);
    }
    Ok(response)
}

fn split_http_response(
    bytes: &[u8],
) -> Result<NativeLoopbackHttpResponse, NativeLoopbackHttpError> {
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or(NativeLoopbackHttpError::InvalidResponse)?;
    let headers = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| NativeLoopbackHttpError::InvalidResponse)?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or(NativeLoopbackHttpError::InvalidResponse)?;
    let headers = headers
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect();
    Ok(NativeLoopbackHttpResponse {
        status,
        headers,
        body: bytes[header_end + 4..].to_vec(),
    })
}

fn redacted_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(key, value)| {
            if key.eq_ignore_ascii_case("set-cookie") || key.eq_ignore_ascii_case("cookie") {
                (key.clone(), "<redacted>".to_string())
            } else {
                (key.clone(), value.clone())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_loopback_http_url_rejects_non_loopback_and_zero_port() {
        assert!(matches!(
            parse_loopback_http_url("https://127.0.0.1:3001"),
            Err(NativeLoopbackHttpError::InvalidUrl)
        ));
        assert!(matches!(
            parse_loopback_http_url("http://192.168.1.10:3001"),
            Err(NativeLoopbackHttpError::InvalidUrl)
        ));
        assert!(matches!(
            parse_loopback_http_url("http://127.0.0.1:0"),
            Err(NativeLoopbackHttpError::InvalidUrl)
        ));
    }

    #[test]
    fn native_loopback_http_response_debug_redacts_cookie_headers() {
        let response = NativeLoopbackHttpResponse {
            status: 200,
            headers: vec![("Set-Cookie".to_string(), "token=secret".to_string())],
            body: b"secret body".to_vec(),
        };
        let debug = format!("{response:?}");

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("token=secret"));
        assert!(!debug.contains("secret body"));
    }
}
