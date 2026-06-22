//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-service-supervisor-contract
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use std::time::Duration;

use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, NativeEndpointReady,
    NativeLoopbackHttpError, NativeLoopbackHttpProbe, loopback_host_from_http_base,
};

use crate::{
    DesktopLocalServiceEntrypointPlan, DesktopNativeSessionCookie, DesktopSessionMaterial,
};

use super::{
    DesktopLocalServiceBootstrapError, DesktopLocalServiceProbe, DesktopLocalServiceProbeOutcome,
    DesktopLocalServiceSessionHandoff, node_role_probe_outcome_from_json,
    session_material_from_auth_status_json,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DesktopLoopbackHttpProbe {
    probe: NativeLoopbackHttpProbe,
}

impl DesktopLoopbackHttpProbe {
    pub fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            probe: NativeLoopbackHttpProbe::new(timeout, max_response_bytes),
        }
    }

    pub fn with_startup_retry(mut self, startup_grace: Duration, retry_interval: Duration) -> Self {
        self.probe = self.probe.with_startup_retry(startup_grace, retry_interval);
        self
    }
}

impl DesktopLocalServiceProbe for DesktopLoopbackHttpProbe {
    fn probe_node_role(
        &mut self,
        plan: &DesktopLocalServiceEntrypointPlan,
    ) -> Result<DesktopLocalServiceProbeOutcome, DesktopLocalServiceBootstrapError> {
        let json = self
            .probe
            .get_json_with_startup_retry(&format!("{}/api/node/role", plan.http_base))
            .map_err(DesktopLocalServiceBootstrapError::from)?;
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
        let json = self
            .probe
            .get_json_with_cookie(
                &format!("{}/api/auth/status", plan.http_base),
                Some(&cookie.request_cookie_header()),
            )
            .map_err(DesktopLocalServiceBootstrapError::from)?;
        session_material_from_auth_status_json(&json)
            .map(|_| DesktopSessionMaterial::bound_with_native_session_cookie(cookie))
    }
}

impl DesktopLoopbackHttpProbe {
    fn issue_native_session_cookie(
        &self,
        plan: &DesktopLocalServiceEntrypointPlan,
        endpoint: &NativeEndpointReady,
        secret: &str,
    ) -> Result<DesktopNativeSessionCookie, DesktopLocalServiceBootstrapError> {
        let response = self
            .probe
            .http_request(
                "POST",
                &format!("{}/api/auth/native-session", plan.http_base),
                &[(NATIVE_SESSION_BOOTSTRAP_HEADER, secret)],
                None,
            )
            .map_err(DesktopLocalServiceBootstrapError::from)?;
        let set_cookie = response
            .header("set-cookie")
            .ok_or(DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid)?;
        let domain = loopback_host_from_http_base(&endpoint.http_base)
            .map_err(DesktopLocalServiceBootstrapError::from)?;
        DesktopNativeSessionCookie::from_set_cookie(set_cookie, &domain)
            .map_err(|_| DesktopLocalServiceBootstrapError::NativeSessionCookieInvalid)
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

impl From<NativeLoopbackHttpError> for DesktopLocalServiceBootstrapError {
    fn from(error: NativeLoopbackHttpError) -> Self {
        match error {
            NativeLoopbackHttpError::InvalidUrl => Self::InvalidProbeUrl,
            NativeLoopbackHttpError::HttpStatus { status } => Self::ProbeHttpStatus { status },
            NativeLoopbackHttpError::ResponseTooLarge => Self::ProbeResponseTooLarge,
            NativeLoopbackHttpError::InvalidResponse => Self::ProbeInvalidResponse,
            NativeLoopbackHttpError::Io(source) => Self::ProbeIo(source),
        }
    }
}
