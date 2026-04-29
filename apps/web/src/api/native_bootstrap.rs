//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{
    NativeEndpointReady, validate_native_endpoint_bases, validate_native_endpoint_ready,
};

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
}

impl NativeBootstrapState {
    pub(super) fn http_base(&self) -> Option<&str> {
        match self {
            Self::Ready(bootstrap) => Some(&bootstrap.http_base),
            Self::Absent | Self::Blocked(_) => None,
        }
    }

    pub(super) fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked(_))
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
) -> NativeBootstrapState {
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
mod tests {
    use super::*;

    fn parse(http_base: &str, ws_base: &str, session_bound: bool) -> NativeBootstrapState {
        parse_native_bootstrap_fields(
            Some(http_base.to_string()),
            Some(ws_base.to_string()),
            Some("main".to_string()),
            Some(session_bound),
        )
    }

    #[test]
    fn accepts_session_bound_loopback_native_bootstrap() {
        assert_eq!(
            parse("http://127.0.0.1:3001/", "ws://localhost:3001/", true),
            NativeBootstrapState::Ready(NativeWebBootstrap {
                http_base: "http://127.0.0.1:3001".to_string(),
                ws_url: "ws://localhost:3001/ws".to_string(),
            })
        );
    }

    #[test]
    fn rejects_native_bootstrap_without_session_binding() {
        assert_eq!(
            parse("http://127.0.0.1:3001", "ws://127.0.0.1:3001", false),
            NativeBootstrapState::Blocked(NativeBootstrapBlocker::SessionNotBound)
        );
    }

    #[test]
    fn rejects_non_loopback_native_bootstrap() {
        assert_eq!(
            parse("http://192.168.1.10:3001", "ws://127.0.0.1:3001", true),
            NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidEndpoint)
        );
    }

    #[test]
    fn rejects_missing_native_bootstrap_fields() {
        assert_eq!(
            parse_native_bootstrap_fields(
                Some("http://127.0.0.1:3001".to_string()),
                None,
                None,
                Some(true),
            ),
            NativeBootstrapState::Blocked(NativeBootstrapBlocker::InvalidShape)
        );
    }
}
