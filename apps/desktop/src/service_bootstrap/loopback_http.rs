//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-service-supervisor-contract
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision

use std::io::Read;
use std::net::{SocketAddr, TcpStream};

use super::DesktopLocalServiceBootstrapError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoopbackHttpTarget {
    pub(super) addr: SocketAddr,
    pub(super) host: String,
    pub(super) host_header: String,
    pub(super) path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoopbackHttpResponse {
    pub(super) status: u16,
    headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl LoopbackHttpResponse {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

pub(super) fn is_retryable_startup_probe_error(error: &DesktopLocalServiceBootstrapError) -> bool {
    matches!(
        error,
        DesktopLocalServiceBootstrapError::ProbeIo(source)
            if matches!(
                source.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::TimedOut
            )
    )
}

pub(super) fn parse_loopback_http_url(
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
        addr: SocketAddr::from(([127, 0, 0, 1], port)),
        host: host.to_string(),
        host_header: authority.to_string(),
        path,
    })
}

pub(super) fn loopback_host_from_http_base(
    url: &str,
) -> Result<String, DesktopLocalServiceBootstrapError> {
    Ok(parse_loopback_http_url(url)?.host)
}

pub(super) fn read_capped_response(
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

pub(super) fn split_http_response(
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
