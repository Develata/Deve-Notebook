// apps\web\src
//! # Deve-Note Web 应用
//! plan_ref:
//!   - 11_ui_design/01_web#single-binary-distribution
//!   - 07_network#web-ws-runtime
//!
//! 这是 Deve-Note 的 WebAssembly 前端，使用 Leptos 框架构建。
//!
//! ## 模块说明
//!
//! - `app`: 主应用组件和布局
//! - `api`: WebSocket 服务，用于后端通信
//! - `components`: 可复用 UI 组件
//! - `editor`: CodeMirror 编辑器集成
//! - `hooks`: 自定义 Leptos Hooks，用于状态管理
//! - `i18n`: 国际化（中英文支持）

// Leptos 框架特定的 clippy 豁免：
// - `unused_unit`: Leptos view! {} 宏返回 () 是框架惯用法
// - `unit_arg`: view! {}.into_any() 是 Leptos 返回空视图的标准方式
// - `clone_on_copy`: Leptos 信号类型实现 Copy 但 clone 更明确表达意图
#![allow(clippy::unused_unit)]
#![allow(clippy::unit_arg)]
#![allow(clippy::clone_on_copy)]

mod api;
mod app;
mod components;
mod context_action;
mod editor;
mod hooks;
mod i18n;
mod runtime;
mod shortcuts;
mod storage;
mod utils;
use app::App;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, JsValue};

fn web_bridge_call_function() -> Option<(JsValue, js_sys::Function)> {
    let window = web_sys::window()?;
    let bridge =
        js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__deveWebBridge")).ok()?;
    let call = js_sys::Reflect::get(&bridge, &JsValue::from_str("call")).ok()?;
    let function = call.dyn_ref::<js_sys::Function>()?.clone();
    Some((bridge, function))
}

fn call_boot_bridge0(name: &str) {
    let Some((bridge, function)) = web_bridge_call_function() else {
        return;
    };
    let _ = function.call1(&bridge, &JsValue::from_str(name));
}

fn call_boot_bridge3(name: &str, title: &str, detail: &str, tone: &str) {
    let Some((bridge, function)) = web_bridge_call_function() else {
        return;
    };
    let _ = function.call4(
        &bridge,
        &JsValue::from_str(name),
        &JsValue::from_str(title),
        &JsValue::from_str(detail),
        &JsValue::from_str(tone),
    );
}

pub fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    tracing::info!("Initializing Deve-Note Web App");

    let Some(window) = web_sys::window() else {
        tracing::error!("Browser window is unavailable; Web app mount skipped");
        return;
    };
    let Some(doc) = window.document() else {
        tracing::error!("Browser document is unavailable; Web app mount skipped");
        return;
    };

    call_boot_bridge3(
        "setBootPanel",
        "Rust/WASM Mounted",
        "Leptos main() has started and mount_to_body is running.",
        "success",
    );

    if let Some(el) = doc.get_element_by_id("loading-overlay") {
        let _ = el.class_list().add_1("hidden");
    }

    mount_to_body(|| {
        view! { <App/> }
    });

    call_boot_bridge0("hideBootPanel");
}
