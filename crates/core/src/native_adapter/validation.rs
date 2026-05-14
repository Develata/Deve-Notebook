//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract

use super::{NativeAdapterSnapshot, NativeAdapterState, NativeEndpointReady};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativeAdapterError {
    #[error("{field} must start with {expected_scheme}://")]
    WrongScheme {
        field: &'static str,
        expected_scheme: &'static str,
    },
    #[error("{field} must include a URL authority")]
    MissingAuthority { field: &'static str },
    #[error("{field} must not include userinfo")]
    UserInfoForbidden { field: &'static str },
    #[error("{field} host must be 127.0.0.1 or localhost")]
    NonLoopbackHost { field: &'static str },
    #[error("{field} port must be a non-zero TCP port")]
    InvalidPort { field: &'static str },
    #[error("{field} must be a base URL without path, query, or fragment")]
    NotBaseUrl { field: &'static str },
    #[error("node_role must not be empty")]
    EmptyNodeRole,
    #[error("session must be bound before the native web shell can become ready")]
    SessionNotBound,
}

pub fn validate_native_endpoint_bases(
    endpoint: &NativeEndpointReady,
) -> Result<(), NativeAdapterError> {
    validate_loopback_base_url("http_base", &endpoint.http_base, "http")?;
    validate_loopback_base_url("ws_base", &endpoint.ws_base, "ws")?;
    if endpoint.node_role.trim().is_empty() {
        return Err(NativeAdapterError::EmptyNodeRole);
    }
    Ok(())
}

pub fn validate_native_endpoint_ready(
    endpoint: &NativeEndpointReady,
) -> Result<(), NativeAdapterError> {
    validate_native_endpoint_bases(endpoint)?;
    if !endpoint.session_bound {
        return Err(NativeAdapterError::SessionNotBound);
    }
    Ok(())
}

pub fn can_load_native_web_shell(snapshot: &NativeAdapterSnapshot) -> bool {
    snapshot
        .endpoint
        .as_ref()
        .is_some_and(|endpoint| validate_native_endpoint_ready(endpoint).is_ok())
        && matches!(
            snapshot.state,
            NativeAdapterState::SessionBound
                | NativeAdapterState::WebShellLoading
                | NativeAdapterState::RuntimeReady
        )
}

pub fn can_show_native_writable_shell(snapshot: &NativeAdapterSnapshot) -> bool {
    snapshot.state.is_writable_candidate()
        && can_load_native_web_shell(snapshot)
        && snapshot.readiness.is_runtime_ready()
}

fn validate_loopback_base_url(
    field: &'static str,
    value: &str,
    expected_scheme: &'static str,
) -> Result<(), NativeAdapterError> {
    let prefix = format!("{expected_scheme}://");
    let Some(rest) = value.strip_prefix(&prefix) else {
        return Err(NativeAdapterError::WrongScheme {
            field,
            expected_scheme,
        });
    };
    if rest.is_empty() {
        return Err(NativeAdapterError::MissingAuthority { field });
    }
    if rest.contains('?') || rest.contains('#') {
        return Err(NativeAdapterError::NotBaseUrl { field });
    }

    let slash_idx = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..slash_idx];
    let path = &rest[slash_idx..];
    if authority.is_empty() {
        return Err(NativeAdapterError::MissingAuthority { field });
    }
    if !path.is_empty() && path != "/" {
        return Err(NativeAdapterError::NotBaseUrl { field });
    }
    if authority.contains('@') {
        return Err(NativeAdapterError::UserInfoForbidden { field });
    }

    let (host, port) = split_host_port(authority);
    if !matches!(host, "127.0.0.1" | "localhost") {
        return Err(NativeAdapterError::NonLoopbackHost { field });
    }
    if let Some(port) = port {
        validate_port(field, port)?;
    }
    Ok(())
}

fn validate_port(field: &'static str, port: &str) -> Result<(), NativeAdapterError> {
    match port.parse::<u16>() {
        Ok(port) if port > 0 => Ok(()),
        _ => Err(NativeAdapterError::InvalidPort { field }),
    }
}

fn split_host_port(authority: &str) -> (&str, Option<&str>) {
    authority
        .rsplit_once(':')
        .filter(|(host, _)| !host.contains(':'))
        .map_or((authority, None), |(host, port)| (host, Some(port)))
}
