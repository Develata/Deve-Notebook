//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-service-supervisor-contract
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision

use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, NativeEndpointReady,
};
use serde_json::Value;

use crate::{
    DesktopLocalServiceEntrypointPlan, DesktopNativeSessionCookie, DesktopSessionMaterial,
};

use super::loopback_http::{
    LoopbackHttpResponse, is_retryable_startup_probe_error, loopback_host_from_http_base,
    parse_loopback_http_url, read_capped_response, split_http_response,
};
use super::{
    DesktopLocalServiceBootstrapError, DesktopLocalServiceProbe, DesktopLocalServiceProbeOutcome,
    DesktopLocalServiceSessionHandoff, node_role_probe_outcome_from_json,
    session_material_from_auth_status_json,
};

const MAX_LOOPBACK_RESPONSE_BYTES: usize = 64 * 1024;
const LOOPBACK_HTTP_TIMEOUT: Duration = Duration::from_secs(1);
const LOOPBACK_HTTP_STARTUP_GRACE: Duration = Duration::from_secs(15);
const LOOPBACK_HTTP_RETRY_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopLoopbackHttpProbe {
    timeout: Duration,
    max_response_bytes: usize,
    startup_grace: Duration,
    retry_interval: Duration,
}

impl Default for DesktopLoopbackHttpProbe {
    fn default() -> Self {
        Self {
            timeout: LOOPBACK_HTTP_TIMEOUT,
            max_response_bytes: MAX_LOOPBACK_RESPONSE_BYTES,
            startup_grace: LOOPBACK_HTTP_STARTUP_GRACE,
            retry_interval: LOOPBACK_HTTP_RETRY_INTERVAL,
        }
    }
}

impl DesktopLoopbackHttpProbe {
    pub fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            timeout,
            max_response_bytes,
            startup_grace: LOOPBACK_HTTP_STARTUP_GRACE,
            retry_interval: LOOPBACK_HTTP_RETRY_INTERVAL,
        }
    }

    pub fn with_startup_retry(mut self, startup_grace: Duration, retry_interval: Duration) -> Self {
        self.startup_grace = startup_grace;
        self.retry_interval = retry_interval;
        self
    }
}

impl DesktopLocalServiceProbe for DesktopLoopbackHttpProbe {
    fn probe_node_role(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
    ) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError> {
        let json =
            self.get_json_with_startup_retry(&format!("{}/api/node/role", plan.http_base))?;
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
    fn get_json_with_startup_retry(
        &self,
        url: &str,
    ) -> Result<Value, DesktopLocalServiceBootstrapError> {
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

fn native_session_secret_from_plan(plan: &DesktopLocalServiceEntrypointPlan) -> Option<&str> {
    plan.spawn_spec
        .env
        .iter()
        .find(|binding| binding.key == NATIVE_SESSION_BOOTSTRAP_SECRET_ENV)
        .map(|binding| binding.value.as_str())
        .filter(|value| !value.trim().is_empty())
}
