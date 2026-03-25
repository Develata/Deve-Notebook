// apps\web\src
//! # Deve-Note Web 应用
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
mod editor;
mod hooks;
mod i18n;
mod shortcuts;
mod storage;
mod utils;
use app::App;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

pub fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    tracing::info!("Initializing Deve-Note Web App");

    let window = web_sys::window().unwrap();
    let doc = window.document().unwrap();

    if let Ok(set_boot_panel) =
        js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("setBootPanel"))
        && let Some(func) = set_boot_panel.dyn_ref::<js_sys::Function>()
    {
        let _ = func.call3(
            &window,
            &wasm_bindgen::JsValue::from_str("Rust/WASM Mounted"),
            &wasm_bindgen::JsValue::from_str(
                "Leptos main() has started and mount_to_body is running.",
            ),
            &wasm_bindgen::JsValue::from_str("success"),
        );
    }

    if let Some(el) = doc.get_element_by_id("loading-overlay") {
        let _ = el.class_list().add_1("hidden");
    }

    mount_to_body(|| {
        view! { <App/> }
    });

    if let Ok(hide_boot_panel) =
        js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("hideBootPanel"))
        && let Some(func) = hide_boot_panel.dyn_ref::<js_sys::Function>()
    {
        let _ = func.call0(&window);
    }
}
