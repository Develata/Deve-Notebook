//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!

use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NativeEndpointReady, NativeLoopbackHttpError,
    NativeLoopbackHttpProbe as CoreLoopbackHttpProbe, loopback_host_from_http_base,
    validate_native_endpoint_bases,
};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};

use super::cookie::MobileNativeSessionCookie;
use super::{MobileEmbeddedBackendError, MobileEmbeddedBackendPlan};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct MobileLoopbackHttpProbe {
    probe: CoreLoopbackHttpProbe,
}

impl MobileLoopbackHttpProbe {
    pub(super) fn probe_node_role(
        &self,
        plan: &MobileEmbeddedBackendPlan,
        cancelled: Option<&AtomicBool>,
    ) -> Result<NativeEndpointReady, MobileEmbeddedBackendError> {
        let json = self
            .probe
            .get_json_with_startup_retry_until(&format!("{}/api/node/role", plan.http_base), || {
                cancelled.is_some_and(|cancelled| cancelled.load(Ordering::Acquire))
            })
            .map_err(MobileEmbeddedBackendError::from)?;
        endpoint_from_node_role_json(plan, &json)
    }

    pub(super) fn bind_native_session(
        &self,
        plan: &MobileEmbeddedBackendPlan,
        endpoint: &NativeEndpointReady,
        secret: &str,
    ) -> Result<MobileNativeSessionCookie, MobileEmbeddedBackendError> {
        let cookie = self.issue_native_session_cookie(plan, endpoint, secret)?;
        self.validate_native_session(plan, &cookie)?;
        Ok(cookie)
    }

    pub(super) fn validate_native_session(
        &self,
        plan: &MobileEmbeddedBackendPlan,
        cookie: &MobileNativeSessionCookie,
    ) -> Result<(), MobileEmbeddedBackendError> {
        let json = self
            .probe
            .get_json_with_cookie(
                &format!("{}/api/auth/status", plan.http_base),
                Some(&cookie.request_cookie_header()),
            )
            .map_err(MobileEmbeddedBackendError::from)?;
        if json
            .get("authenticated")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            Ok(())
        } else {
            Err(MobileEmbeddedBackendError::NativeSessionHandoffFailed)
        }
    }

    fn issue_native_session_cookie(
        &self,
        plan: &MobileEmbeddedBackendPlan,
        endpoint: &NativeEndpointReady,
        secret: &str,
    ) -> Result<MobileNativeSessionCookie, MobileEmbeddedBackendError> {
        let response = self
            .probe
            .http_request(
                "POST",
                &format!("{}/api/auth/native-session", plan.http_base),
                &[(NATIVE_SESSION_BOOTSTRAP_HEADER, secret)],
                None,
            )
            .map_err(MobileEmbeddedBackendError::from)?;
        let set_cookie = response
            .header("set-cookie")
            .ok_or(MobileEmbeddedBackendError::NativeSessionCookieInvalid)?;
        let domain = loopback_host_from_http_base(&endpoint.http_base)
            .map_err(MobileEmbeddedBackendError::from)?;
        MobileNativeSessionCookie::from_set_cookie(set_cookie, &domain)
    }
}

fn endpoint_from_node_role_json(
    plan: &MobileEmbeddedBackendPlan,
    json: &Value,
) -> Result<NativeEndpointReady, MobileEmbeddedBackendError> {
    let node_role = non_empty_value_field(json, "role")?;
    let endpoint = match json.pointer("/native_service/endpoint") {
        Some(Value::Object(endpoint)) => endpoint_from_json(endpoint)?,
        _ => NativeEndpointReady {
            http_base: plan.http_base.clone(),
            ws_base: plan.ws_base.clone(),
            node_role: node_role.to_string(),
            session_bound: false,
        },
    };
    validate_native_endpoint_bases(&endpoint)?;
    if endpoint.http_base != plan.http_base || endpoint.ws_base != plan.ws_base {
        return Err(MobileEmbeddedBackendError::ProbeInvalidResponse);
    }
    Ok(endpoint)
}

fn endpoint_from_json(
    endpoint: &serde_json::Map<String, Value>,
) -> Result<NativeEndpointReady, MobileEmbeddedBackendError> {
    Ok(NativeEndpointReady {
        http_base: string_field(endpoint, "http_base")?,
        ws_base: string_field(endpoint, "ws_base")?,
        node_role: non_empty_string_field(endpoint, "node_role")?,
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

fn non_empty_string_field(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<String, MobileEmbeddedBackendError> {
    let value = string_field(object, field)?;
    if value.trim().is_empty() {
        return Err(MobileEmbeddedBackendError::ProbeInvalidResponse);
    }
    Ok(value)
}

fn non_empty_value_field<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a str, MobileEmbeddedBackendError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or(MobileEmbeddedBackendError::ProbeInvalidResponse)
}

impl From<NativeLoopbackHttpError> for MobileEmbeddedBackendError {
    fn from(error: NativeLoopbackHttpError) -> Self {
        match error {
            NativeLoopbackHttpError::InvalidUrl => Self::InvalidProbeUrl,
            NativeLoopbackHttpError::HttpStatus { status } => Self::ProbeHttpStatus { status },
            NativeLoopbackHttpError::ResponseTooLarge => Self::ProbeResponseTooLarge,
            NativeLoopbackHttpError::InvalidResponse => Self::ProbeInvalidResponse,
            NativeLoopbackHttpError::Cancelled => Self::LifecycleTransitionCancelled,
            NativeLoopbackHttpError::Io(source) => Self::ProbeIo(source),
        }
    }
}

#[cfg(test)]
#[path = "http_test.rs"]
mod tests;
