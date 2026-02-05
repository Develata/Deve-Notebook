// apps/web/src/components/outline_render/katex.rs
//! # KaTeX Helpers

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

pub fn render_katex_to_string(expr: &str) -> Option<String> {
    let window = web_sys::window()?;
    let katex = Reflect::get(&window, &JsValue::from_str("katex")).ok()?;
    if katex.is_undefined() {
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
