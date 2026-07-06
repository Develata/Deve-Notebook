//! plan_ref:
//!   - 15_settings#native-host-local-backend-preference

use js_sys::{Array, Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

const NATIVE_BACKEND_CONFIG_FACADE: &str = "__DEVE_NATIVE_BACKEND_CONFIG__";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeBackendConfig {
    pub available: bool,
    pub mode: String,
    pub remote_url: String,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeBackendValidation {
    pub available: bool,
    pub ok: bool,
    pub https_origin: String,
    pub node_role: String,
    pub error_message: Option<String>,
}

pub async fn get_native_backend_config() -> NativeBackendConfig {
    match call_facade_method("getConfig", Array::new()).await {
        Ok(value) => parse_config_response(&value),
        Err(error) => NativeBackendConfig::unavailable(Some(error)),
    }
}

pub async fn save_native_backend_remote(remote_url: String) -> NativeBackendValidation {
    let args = Array::new();
    args.push(&JsValue::from_str(&remote_url));
    match call_facade_method("saveRemote", args).await {
        Ok(value) => parse_validation_response(&value),
        Err(error) => NativeBackendValidation::unavailable(Some(error)),
    }
}

pub async fn switch_native_backend_local() -> NativeBackendConfig {
    match call_facade_method("switchLocal", Array::new()).await {
        Ok(value) => parse_config_response(&value),
        Err(error) => NativeBackendConfig::unavailable(Some(error)),
    }
}

async fn call_facade_method(method: &str, args: Array) -> Result<JsValue, String> {
    let facade =
        native_backend_facade().ok_or_else(|| "native backend bridge unavailable".to_string())?;
    let function = Reflect::get(&facade, &JsValue::from_str(method))
        .map_err(js_error_to_string)?
        .dyn_into::<Function>()
        .map_err(|_| format!("native backend bridge method {method} unavailable"))?;
    let value = function.apply(&facade, &args).map_err(js_error_to_string)?;
    let promise = Promise::resolve(&value);
    JsFuture::from(promise).await.map_err(js_error_to_string)
}

fn native_backend_facade() -> Option<JsValue> {
    let window = web_sys::window()?;
    let bridge = Reflect::get(window.as_ref(), &JsValue::from_str("__deveWebBridge")).ok()?;
    let get = Reflect::get(&bridge, &JsValue::from_str("get"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    get.call1(&bridge, &JsValue::from_str(NATIVE_BACKEND_CONFIG_FACADE))
        .ok()
        .filter(|value| !value.is_null() && !value.is_undefined())
}

fn parse_config_response(value: &JsValue) -> NativeBackendConfig {
    if !bool_field(value, "available") {
        return NativeBackendConfig::unavailable(string_field(value, "error"));
    }
    let config = object_field(value, "value").unwrap_or_else(|| value.clone());
    NativeBackendConfig {
        available: true,
        mode: string_field(&config, "mode").unwrap_or_else(|| "local".to_string()),
        remote_url: string_field(&config, "remote_url")
            .or_else(|| string_field(&config, "remoteUrl"))
            .unwrap_or_default(),
        error_message: None,
    }
}

fn parse_validation_response(value: &JsValue) -> NativeBackendValidation {
    if !bool_field(value, "available") {
        return NativeBackendValidation::unavailable(string_field(value, "error"));
    }
    let result = object_field(value, "value").unwrap_or_else(|| value.clone());
    NativeBackendValidation {
        available: true,
        ok: bool_field(&result, "ok"),
        https_origin: string_field(&result, "https_origin")
            .or_else(|| string_field(&result, "httpsOrigin"))
            .unwrap_or_default(),
        node_role: string_field(&result, "node_role")
            .or_else(|| string_field(&result, "nodeRole"))
            .unwrap_or_default(),
        error_message: string_field(&result, "error"),
    }
}

fn object_field(value: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .filter(|value| !value.is_null() && !value.is_undefined())
}

fn string_field(value: &JsValue, key: &str) -> Option<String> {
    object_field(value, key).and_then(|value| value.as_string())
}

fn bool_field(value: &JsValue, key: &str) -> bool {
    object_field(value, key)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn js_error_to_string(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "native backend bridge call failed".to_string())
}

impl NativeBackendConfig {
    fn unavailable(error_message: Option<String>) -> Self {
        Self {
            available: false,
            mode: "local".to_string(),
            remote_url: String::new(),
            error_message,
        }
    }
}

impl NativeBackendValidation {
    fn unavailable(error_message: Option<String>) -> Self {
        Self {
            available: false,
            ok: false,
            https_origin: String::new(),
            node_role: String::new(),
            error_message,
        }
    }
}

#[cfg(test)]
mod tests {
    fn source_before_tests() -> &'static str {
        include_str!("native_backend_bridge.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("source before tests")
    }

    #[test]
    fn native_backend_facade_reads_through_bridge_registry() {
        let source = source_before_tests();

        assert!(source.contains("NATIVE_BACKEND_CONFIG_FACADE"));
        assert!(source.contains("\"__deveWebBridge\""));
        assert!(source.contains("\"get\""));
        assert!(
            source.contains("&JsValue::from_str(NATIVE_BACKEND_CONFIG_FACADE)"),
            "native backend config facade must be requested through the bridge registry"
        );
        assert!(
            source
                .match_indices("Reflect::get(window.as_ref(),")
                .all(|(index, _)| {
                    let lookup_tail = &source[index..source.len().min(index + 240)];
                    !lookup_tail.contains(super::NATIVE_BACKEND_CONFIG_FACADE)
                        && !lookup_tail.contains("__DEVE_NATIVE_BACKEND_CONFIG__")
                }),
            "native backend bridge must not read the facade directly from window"
        );
    }
}
