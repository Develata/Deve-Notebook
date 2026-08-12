//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{
    NativeEndpointReady, validate_native_endpoint_bases, validate_native_endpoint_ready,
};

use super::ConnectionStatus;

#[cfg(target_arch = "wasm32")]
const NATIVE_BOOTSTRAP_GLOBAL: &str = "__DEVE_NATIVE_BOOTSTRAP";
const NATIVE_PLATFORM_LIFECYCLE_AUTHORITY: &str = "native";

#[cfg(target_arch = "wasm32")]
const FORBIDDEN_NATIVE_BOOTSTRAP_FIELDS: &[&str] = &[
    "reason",
    "retryable",
    "token",
    "p2p_token",
    "p2p_token_material",
    "session_secret",
    "session_material",
    "session_token",
    "bootstrap_secret",
    "auth_material",
    "auth_token",
    "repo_write",
    "repo_write_permission",
    "writer_ready",
    "scope_nonce",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeWebBootstrap {
    pub http_base: String,
    pub ws_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) enum NativeBootstrapState {
    Absent,
    Ready(NativeWebBootstrap),
    Blocked(NativeBootstrapBlocker),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) enum NativeBootstrapBlocker {
    InvalidShape,
    InvalidEndpoint,
    SessionNotBound,
    ServiceOffline,
    ForegroundReprobe,
    SessionInvalid,
}

impl NativeBootstrapState {
    pub(super) fn blocked_status(&self) -> Option<ConnectionStatus> {
        match self {
            Self::Blocked(NativeBootstrapBlocker::InvalidShape)
            | Self::Blocked(NativeBootstrapBlocker::InvalidEndpoint) => {
                Some(ConnectionStatus::NativeBootstrapInvalid)
            }
            Self::Blocked(NativeBootstrapBlocker::SessionNotBound) => {
                Some(ConnectionStatus::NativeSessionPending)
            }
            Self::Blocked(NativeBootstrapBlocker::ServiceOffline) => {
                Some(ConnectionStatus::NativeServiceOffline)
            }
            Self::Blocked(NativeBootstrapBlocker::ForegroundReprobe) => {
                Some(ConnectionStatus::NativeReprobeRequired)
            }
            Self::Blocked(NativeBootstrapBlocker::SessionInvalid) => {
                Some(ConnectionStatus::Unauthorized)
            }
            Self::Absent | Self::Ready(_) => None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn read_native_bootstrap() -> NativeBootstrapState {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let Some(window) = web_sys::window() else {
        return NativeBootstrapState::Absent;
    };
    let location = window.location();
    let hostname = location.hostname().ok();
    let protocol = location.protocol().ok();
    let Ok(value) = Reflect::get(window.as_ref(), &JsValue::from_str(NATIVE_BOOTSTRAP_GLOBAL))
    else {
        return absent_native_bootstrap_state(hostname.as_deref(), protocol.as_deref());
    };
    if value.is_undefined() || value.is_null() {
        return absent_native_bootstrap_state(hostname.as_deref(), protocol.as_deref());
    }

    let forbidden_material_present = forbidden_native_bootstrap_material_present(&value);
    parse_native_bootstrap_fields(
        js_string_field(&value, "http_base"),
        js_string_field(&value, "ws_base"),
        js_string_field(&value, "node_role"),
        js_bool_field(&value, "session_bound"),
        js_string_field(&value, "service_state"),
        forbidden_material_present,
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn read_native_bootstrap() -> NativeBootstrapState {
    NativeBootstrapState::Absent
}

pub(crate) fn current_native_bootstrap_blocked_status() -> Option<ConnectionStatus> {
    read_native_bootstrap().blocked_status()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn current_native_platform_lifecycle_authority() -> bool {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let Some(window) = web_sys::window() else {
        return false;
    };
    let location = window.location();
    let Ok(hostname) = location.hostname() else {
        return false;
    };
    let Ok(protocol) = location.protocol() else {
        return false;
    };
    let Ok(value) = Reflect::get(window.as_ref(), &JsValue::from_str(NATIVE_BOOTSTRAP_GLOBAL))
    else {
        return false;
    };
    native_platform_lifecycle_authority(
        &hostname,
        &protocol,
        js_string_field(&value, "platform_lifecycle_authority").as_deref(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn current_native_platform_lifecycle_authority() -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn current_bundled_local_backend() -> bool {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let Some(window) = web_sys::window() else {
        return false;
    };
    let location = window.location();
    let Ok(hostname) = location.hostname() else {
        return false;
    };
    let Ok(protocol) = location.protocol() else {
        return false;
    };
    let Ok(bootstrap) = Reflect::get(window.as_ref(), &JsValue::from_str(NATIVE_BOOTSTRAP_GLOBAL))
    else {
        return false;
    };
    let capabilities = Reflect::get(&bootstrap, &JsValue::from_str("capabilities")).ok();
    bundled_local_backend_presentation(
        &hostname,
        &protocol,
        capabilities
            .as_ref()
            .and_then(|value| js_bool_field(value, "backend_preference_control")),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn current_bundled_local_backend() -> bool {
    false
}

fn bundled_local_backend_presentation(
    hostname: &str,
    protocol: &str,
    backend_preference_control: Option<bool>,
) -> bool {
    is_native_shell_origin(hostname, protocol) && backend_preference_control == Some(true)
}

fn native_platform_lifecycle_authority(
    hostname: &str,
    protocol: &str,
    authority: Option<&str>,
) -> bool {
    is_native_shell_origin(hostname, protocol)
        && authority == Some(NATIVE_PLATFORM_LIFECYCLE_AUTHORITY)
}

#[cfg(target_arch = "wasm32")]
fn js_string_field(value: &wasm_bindgen::JsValue, key: &str) -> Option<String> {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|field| field.as_string())
}

#[cfg(target_arch = "wasm32")]
fn js_bool_field(value: &wasm_bindgen::JsValue, key: &str) -> Option<bool> {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|field| field.as_bool())
}

#[cfg(target_arch = "wasm32")]
fn forbidden_native_bootstrap_material_present(value: &wasm_bindgen::JsValue) -> bool {
    FORBIDDEN_NATIVE_BOOTSTRAP_FIELDS
        .iter()
        .any(|key| js_field_present(value, key))
}

#[cfg(target_arch = "wasm32")]
fn js_field_present(value: &wasm_bindgen::JsValue, key: &str) -> bool {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    Reflect::has(value, &JsValue::from_str(key)).unwrap_or(false)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn parse_native_bootstrap_fields(
    http_base: Option<String>,
    ws_base: Option<String>,
    node_role: Option<String>,
    session_bound: Option<bool>,
    service_state: Option<String>,
    forbidden_material_present: bool,
) -> NativeBootstrapState {
    if forbidden_material_present {
        return NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape);
    }

    let has_endpoint_fields =
        http_base.is_some() || ws_base.is_some() || node_role.is_some() || session_bound.is_some();
    if let Some(service_state) = service_state {
        let recovery_blocker = match service_state.as_str() {
            "service_offline" => Some(NativeBootstrapBlocker::ServiceOffline),
            "foreground_reprobe" => Some(NativeBootstrapBlocker::ForegroundReprobe),
            "session_invalid" => Some(NativeBootstrapBlocker::SessionInvalid),
            "endpoint_ready" | "session_bound" | "runtime_ready" => None,
            _ => return NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape),
        };
        if let Some(blocker) = recovery_blocker {
            if has_endpoint_fields {
                return NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape);
            }
            return NativeBootstrapState::Blocked(blocker);
        }
    }

    let (Some(http_base), Some(ws_base), Some(node_role), Some(session_bound)) =
        (http_base, ws_base, node_role, session_bound)
    else {
        return NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape);
    };

    let endpoint = NativeEndpointReady {
        http_base,
        ws_base,
        node_role,
        session_bound,
    };

    if !endpoint.session_bound {
        return match validate_native_endpoint_bases(&endpoint) {
            Ok(()) => NativeBootstrapState::Blocked(NativeBootstrapBlocker::SessionNotBound),
            Err(_) => NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidEndpoint),
        };
    }
    if validate_native_endpoint_ready(&endpoint).is_err() {
        return NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidEndpoint);
    }

    NativeBootstrapState::Ready(NativeWebBootstrap {
        http_base: trim_base(endpoint.http_base),
        ws_url: format!("{}/ws", trim_base(endpoint.ws_base)),
    })
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn absent_native_bootstrap_state(
    hostname: Option<&str>,
    protocol: Option<&str>,
) -> NativeBootstrapState {
    if is_native_shell_origin(hostname.unwrap_or_default(), protocol.unwrap_or_default()) {
        NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape)
    } else {
        NativeBootstrapState::Absent
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn is_native_shell_origin(hostname: &str, protocol: &str) -> bool {
    (protocol == "http:" && hostname == "tauri.localhost")
        || (protocol == "tauri:" && hostname == "localhost")
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn trim_base(base: String) -> String {
    base.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests;
