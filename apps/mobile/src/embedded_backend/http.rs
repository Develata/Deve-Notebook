use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_HEADER, NativeEndpointReady, NativeLoopbackHttpError,
    NativeLoopbackHttpProbe as CoreLoopbackHttpProbe, loopback_host_from_http_base,
    validate_native_endpoint_bases,
};
use serde_json::Value;

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
    ) -> Result<NativeEndpointReady, MobileEmbeddedBackendError> {
        let json = self
            .probe
            .get_json_with_startup_retry(&format!("{}/api/node/role", plan.http_base))
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
            Ok(cookie)
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

impl From<NativeLoopbackHttpError> for MobileEmbeddedBackendError {
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
