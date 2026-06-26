//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use super::{
    NativeAdapterSnapshot, NativeAdapterState, NativeBackendMode, NativeBackendPreference,
    NativeEndpointReady, NativeRemoteTarget, NativeShellMode,
};
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
    #[error("remote_browser target must be an https origin")]
    RemoteTargetMustBeHttpsOrigin,
    #[error("remote backend preference requires a remote_url")]
    MissingRemoteBackendUrl,
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

pub fn validate_native_remote_target(
    target: &NativeRemoteTarget,
) -> Result<(), NativeAdapterError> {
    validate_https_origin_url("https_origin", &target.https_origin)
}

pub fn validate_native_backend_preference(
    preference: &NativeBackendPreference,
) -> Result<(), NativeAdapterError> {
    native_shell_mode_for_backend_preference(preference).map(|_| ())
}

pub fn native_shell_mode_for_backend_preference(
    preference: &NativeBackendPreference,
) -> Result<NativeShellMode, NativeAdapterError> {
    match preference.mode {
        NativeBackendMode::Local => Ok(NativeShellMode::LocalBackend),
        NativeBackendMode::Remote => {
            let remote_url = preference
                .remote_url
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or(NativeAdapterError::MissingRemoteBackendUrl)?;
            let target = NativeRemoteTarget {
                https_origin: remote_url.to_string(),
            };
            validate_native_remote_target(&target)?;
            Ok(NativeShellMode::RemoteBrowser { target })
        }
    }
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

fn validate_https_origin_url(field: &'static str, value: &str) -> Result<(), NativeAdapterError> {
    if value.trim() != value {
        return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return Err(NativeAdapterError::WrongScheme {
            field,
            expected_scheme: "https",
        });
    };
    if rest.is_empty() {
        return Err(NativeAdapterError::MissingAuthority { field });
    }
    if rest.contains('?')
        || rest.contains('#')
        || rest.contains('\\')
        || rest
            .chars()
            .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
    }

    let slash_idx = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..slash_idx];
    let path = &rest[slash_idx..];
    if authority.is_empty() {
        return Err(NativeAdapterError::MissingAuthority { field });
    }
    if authority.contains('@') {
        return Err(NativeAdapterError::UserInfoForbidden { field });
    }
    if !path.is_empty() {
        return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
    }

    let (host, port) = split_https_authority(field, authority)?;
    validate_https_host(field, host)?;
    if let Some(port) = port {
        validate_port(field, port)?;
    }
    Ok(())
}

fn split_https_authority<'a>(
    field: &'static str,
    authority: &'a str,
) -> Result<(&'a str, Option<&'a str>), NativeAdapterError> {
    if authority.starts_with('[') {
        let Some(close_idx) = authority.find(']') else {
            return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
        };
        let host = &authority[..=close_idx];
        let suffix = &authority[close_idx + 1..];
        return if suffix.is_empty() {
            Ok((host, None))
        } else if let Some(port) = suffix.strip_prefix(':') {
            if port.is_empty() {
                Err(NativeAdapterError::InvalidPort { field })
            } else {
                Ok((host, Some(port)))
            }
        } else {
            Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin)
        };
    }

    if authority.contains('[') || authority.contains(']') || authority.matches(':').count() > 1 {
        return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
    }
    match authority.rsplit_once(':') {
        Some((_host, "")) => Err(NativeAdapterError::InvalidPort { field }),
        Some((host, port)) => Ok((host, Some(port))),
        None => Ok((authority, None)),
    }
}

fn validate_https_host(field: &'static str, host: &str) -> Result<(), NativeAdapterError> {
    if host.is_empty() {
        return Err(NativeAdapterError::MissingAuthority { field });
    }
    if host.starts_with('[') {
        if !host.ends_with(']') || host.len() <= 2 {
            return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
        }
        let inner = &host[1..host.len() - 1];
        if !inner.contains(':')
            || inner
                .bytes()
                .any(|byte| !(byte.is_ascii_hexdigit() || matches!(byte, b':' | b'.')))
        {
            return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
        }
        return Ok(());
    }

    for label in host.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(NativeAdapterError::RemoteTargetMustBeHttpsOrigin);
        }
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
