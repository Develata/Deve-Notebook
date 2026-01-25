// apps\web\src
//! # App Component (App 组件)
//!
//! 本模块包含根 `App` 组件。
//! 主布局逻辑已移动至 `components/main_layout.rs`。
//!
//! ## 结构说明
//!
//! - `App`: 根组件，提供语言环境上下文。
//!

use crate::components::main_layout::MainLayout;
use crate::i18n::Locale;
use leptos::prelude::*;

/// 根应用程序组件
///
/// 初始化全局状态 (Locale) 并渲染 MainLayout。
#[component]
pub fn App() -> impl IntoView {
    // 全局语言环境状态
    let locale = RwSignal::new(Locale::default());
    provide_context(locale);

    view! {
        <MainLayout/>
    }
}
