//! plan_ref:
//!   - 11_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design_03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{
    NativeEndpointReady, validate_native_endpoint_bases, validate_native_endpoint_ready,
};

use super::ConnectionStatus;

#[cfg(target_arch = "wasm32")]
const NATIVE_BOOTSTRAP_GLOBAL: &str = "__DEVE_NATIVE_BOOTSTRAP";

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
    pub(super) fn http_base(&self) -> Option<&str> {
        match self {
            Self::Ready(bootstrap) => Some(&bootstrap.http_base),
            Self::Absent | Self::Blocked(_) => None,
        }
    }

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
    let Ok(value) = Reflect::get(window.as_ref(), &JsValue::from_str(NATIVE_BOOTSTRAP_GLOBAL))
    else {
        return NativeBootstrapState::Absent;
    };
    if value.is_undefined() || value.is_null() {
        return NativeBootstrapState::Absent;
    }

    parse_native_bootstrap_fields(
        js_string_field(&value, "http_base"),
        js_string_field(&value, "ws_base"),
        js_string_field(&value, "node_role"),
        js_bool_field(&value, "session_bound"),
        js_string_field(&value, "service_state"),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn read_native_bootstrap() -> NativeBootstrapState {
    NativeBootstrapState::Absent
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

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn parse_native_bootstrap_fields(
    http_base: Option<String>,
    ws_base: Option<String>,
    node_role: Option<String>,
    session_bound: Option<bool>,
    service_state: Option<String>,
) -> NativeBootstrapState {
    if let Some(service_state) = service_state {
        match service_state.as_str() {
            "service_offline" => {
                return NativeBootstrapState::Blocked(NativeBootstrapBlocker::ServiceOffline);
            }
            "foreground_reprobe" => {
                return NativeBootstrapState::Blocked(NativeBootstrapBlocker::ForegroundReprobe);
            }
            "session_invalid" => {
                return NativeBootstrapState::Blocked(NativeBootstrapBlocker::SessionInvalid);
            }
            "endpoint_ready" | "session_bound" | "runtime_ready" => {}
            _ => return NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape),
        }
    }

    let (Some(http_base), Some(ws_base), Some(session_bound)) = (http_base, ws_base, session_bound)
    else {
        return NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape);
    };

    let endpoint = NativeEndpointReady {
        http_base,
        ws_base,
        node_role: node_role.unwrap_or_else(|| "native".to_string()),
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
fn trim_base(base: String) -> String {
    base.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests;
