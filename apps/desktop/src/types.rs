//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract

use std::fmt;

use deve_core::native_adapter::{
    NativeAdapterError, NativeEndpointReady, NativeProcessAdapterError,
    NativeProcessAdapterSnapshot, NativeRuntimeReadiness, NativeServiceOffline,
    NativeServiceRestarting, NativeServiceSupervisorError, NativeServiceSupervisorSnapshot,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopServiceState {
    ColdStart,
    ServiceStarting,
    EndpointBound,
    SessionBound,
    WebShellLoading,
    RuntimeReady,
    ServiceRestarting,
    ServiceOffline,
    SessionInvalid,
    ForegroundReprobe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellSnapshot {
    pub state: DesktopServiceState,
    pub endpoint: Option<NativeEndpointReady>,
    pub readiness: NativeRuntimeReadiness,
    pub offline: Option<NativeServiceOffline>,
    pub restarting: Option<NativeServiceRestarting>,
    pub supervisor: NativeServiceSupervisorSnapshot,
    pub process_adapter: NativeProcessAdapterSnapshot,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopSessionMaterial {
    pub(super) bound: bool,
    native_session_cookie: Option<DesktopNativeSessionCookie>,
}

impl DesktopSessionMaterial {
    pub fn bound() -> Self {
        Self {
            bound: true,
            native_session_cookie: None,
        }
    }

    pub fn pending() -> Self {
        Self {
            bound: false,
            native_session_cookie: None,
        }
    }

    pub fn bound_with_native_session_cookie(cookie: DesktopNativeSessionCookie) -> Self {
        Self {
            bound: true,
            native_session_cookie: Some(cookie),
        }
    }

    pub fn native_session_cookie(&self) -> Option<&DesktopNativeSessionCookie> {
        self.native_session_cookie.as_ref()
    }
}

impl fmt::Debug for DesktopSessionMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopSessionMaterial")
            .field("bound", &self.bound)
            .field("native_session_cookie", &self.native_session_cookie)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DesktopNativeSessionCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    http_only: bool,
    same_site: String,
}

impl DesktopNativeSessionCookie {
    pub fn from_set_cookie(set_cookie: &str, domain: &str) -> Result<Self, DesktopShellError> {
        let mut parts = set_cookie.split(';').map(str::trim);
        let Some(name_value) = parts.next() else {
            return Err(DesktopShellError::NativeSessionCookieInvalid);
        };
        let Some((name, value)) = name_value.split_once('=') else {
            return Err(DesktopShellError::NativeSessionCookieInvalid);
        };
        let name = name.trim();
        let value = value.trim();
        if name != "token" || value.is_empty() || !is_loopback_cookie_domain(domain) {
            return Err(DesktopShellError::NativeSessionCookieInvalid);
        }

        let mut cookie = Self {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: String::new(),
        };
        for attr in parts {
            let lower = attr.to_ascii_lowercase();
            if lower == "httponly" {
                cookie.http_only = true;
            } else if lower == "secure" {
                cookie.secure = true;
            } else if let Some((key, value)) = attr.split_once('=') {
                if key.eq_ignore_ascii_case("path") {
                    cookie.path = value.trim().to_string();
                } else if key.eq_ignore_ascii_case("samesite") {
                    cookie.same_site = value.trim().to_string();
                }
            }
        }
        if !cookie.http_only || !cookie.secure || !cookie.same_site.eq_ignore_ascii_case("none") {
            return Err(DesktopShellError::NativeSessionCookieInvalid);
        }
        Ok(cookie)
    }

    #[cfg(feature = "native-packaging")]
    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn secure(&self) -> bool {
        self.secure
    }

    pub fn http_only(&self) -> bool {
        self.http_only
    }

    pub fn same_site(&self) -> &str {
        &self.same_site
    }

    #[cfg(feature = "native-packaging")]
    pub(crate) fn request_cookie_header(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

fn is_loopback_cookie_domain(domain: &str) -> bool {
    matches!(domain.trim(), "127.0.0.1" | "localhost")
}

impl fmt::Debug for DesktopNativeSessionCookie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopNativeSessionCookie")
            .field("name", &self.name)
            .field("value", &"<redacted>")
            .field("domain", &self.domain)
            .field("path", &self.path)
            .field("secure", &self.secure)
            .field("http_only", &self.http_only)
            .field("same_site", &self.same_site)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopBootstrap {
    pub http_base: String,
    pub ws_base: String,
    pub node_role: String,
    pub session_bound: bool,
}

impl DesktopBootstrap {
    pub fn script_source(&self) -> Result<String, DesktopShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!("window.__DEVE_NATIVE_BOOTSTRAP={payload};"))
    }

    pub fn script_tag(&self) -> Result<String, DesktopShellError> {
        Ok(format!("<script>{}</script>", self.script_source()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopRecoveryBootstrap {
    pub service_state: &'static str,
}

impl DesktopRecoveryBootstrap {
    pub fn script_source(&self) -> Result<String, DesktopShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!("window.__DEVE_NATIVE_BOOTSTRAP={payload};"))
    }

    pub fn script_tag(&self) -> Result<String, DesktopShellError> {
        Ok(format!("<script>{}</script>", self.script_source()?))
    }
}

#[derive(Debug, Error)]
pub enum DesktopShellError {
    #[error("desktop service endpoint is invalid: {0}")]
    InvalidEndpoint(#[from] NativeAdapterError),
    #[error("desktop session is not bound")]
    SessionNotBound,
    #[error("desktop service is offline: {reason}")]
    ServiceOffline { reason: String },
    #[error("desktop session is invalid")]
    SessionInvalid,
    #[error("desktop native session cookie is invalid")]
    NativeSessionCookieInvalid,
    #[error("desktop foreground reprobe is required before loading writable shell")]
    ForegroundReprobeRequired,
    #[error("desktop service supervisor rejected transition: {0}")]
    Supervisor(#[from] NativeServiceSupervisorError),
    #[error("desktop process adapter rejected transition: {0}")]
    ProcessAdapter(#[from] NativeProcessAdapterError),
    #[error("failed to serialize desktop bootstrap: {0}")]
    BootstrapSerialize(#[from] serde_json::Error),
}
