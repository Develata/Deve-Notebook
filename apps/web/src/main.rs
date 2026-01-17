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

mod app;
mod editor;
mod api;
mod components;
mod i18n;
mod hooks;
mod shortcuts;
use app::App;
use leptos::prelude::*;

pub fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    tracing::info!("Initializing Deve-Note Web App");
    
    // 挂载时手动隐藏覆盖层，以防止在未选择文档时卡住
    let window = web_sys::window().unwrap();
    let doc = window.document().unwrap();
    if let Some(el) = doc.get_element_by_id("loading-overlay") {
        let _ = el.class_list().add_1("hidden");
    }

    mount_to_body(|| {
        view! { <App/> }
    })
}
