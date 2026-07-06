// apps/web/src/components/outline_render/katex.rs
//! plan_ref:
//!   - 10_rendering#markdown-render-whitelist
//!
//! # KaTeX Helpers

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

const WEB_BRIDGE_REGISTRY: &str = "__deveWebBridge";
const KATEX_FACADE: &str = "__deveKatex";

pub fn render_katex_to_string(expr: &str) -> Option<String> {
    let window = web_sys::window()?;
    let bridge = Reflect::get(window.as_ref(), &JsValue::from_str(WEB_BRIDGE_REGISTRY)).ok()?;
    let get = Reflect::get(&bridge, &JsValue::from_str("get"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    let katex = get.call1(&bridge, &JsValue::from_str(KATEX_FACADE)).ok()?;
    if katex.is_null() || katex.is_undefined() {
        return None;
    }
    let render = Reflect::get(&katex, &JsValue::from_str("renderToString")).ok()?;
    let func: Function = render.dyn_into().ok()?;
    let options = Object::new();
    let _ = Reflect::set(
        &options,
        &JsValue::from_str("throwOnError"),
        &JsValue::FALSE,
    );
    let _ = Reflect::set(&options, &JsValue::from_str("displayMode"), &JsValue::FALSE);
    let html = func
        .call2(&katex, &JsValue::from_str(expr), &options)
        .ok()?;
    html.as_string()
}

#[cfg(test)]
mod tests {
    fn source_before_tests() -> &'static str {
        include_str!("katex.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("source before tests")
    }

    #[test]
    fn outline_katex_reads_through_bridge_registry() {
        let source = source_before_tests();

        assert!(source.contains("\"__deveWebBridge\""));
        assert!(source.contains("\"get\""));
        assert!(source.contains("\"__deveKatex\""));
        assert!(source.contains("\"renderToString\""));
        assert!(!source.contains("JsValue::from_str(\"katex\")"));
        assert!(!source.contains("Reflect::get(&window"));
    }
}
