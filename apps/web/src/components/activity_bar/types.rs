// apps/web/src/components/activity_bar/types.rs
//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # SidebarView 枚举定义
//!
//! 侧边栏视图类型，在 ActivityBar、Sidebar、Layout 等组件间共享。

use crate::components::icons;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum SidebarView {
    #[default]
    Explorer, // 资源管理器
    Search,        // 搜索
    SourceControl, // 源代码管理 (Git)
    Extensions,    // 扩展
}

impl SidebarView {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Explorer,
            Self::Search,
            Self::SourceControl,
            Self::Extensions,
        ]
    }

    pub fn title(&self, locale: Locale) -> &'static str {
        match self {
            Self::Explorer => t::sidebar::explorer(locale),
            Self::Search => t::sidebar::search(locale),
            Self::SourceControl => t::sidebar::source_control(locale),
            Self::Extensions => t::sidebar::extensions(locale),
        }
    }

    /// 返回对应的 Lucide 图标组件视图。
    pub fn icon_view(&self, class: &str) -> AnyView {
        let cls = class.to_string();
        match self {
            Self::Explorer => view! { <icons::File class=cls/> }.into_any(),
            Self::Search => view! { <icons::Search class=cls/> }.into_any(),
            Self::SourceControl => view! { <icons::SourceControl class=cls/> }.into_any(),
            Self::Extensions => view! { <icons::LayoutGrid class=cls/> }.into_any(),
        }
    }
}
