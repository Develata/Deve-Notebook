// apps/web/src/components/activity_bar/types.rs
//! # SidebarView 枚举定义
//!
//! 侧边栏视图类型，在 ActivityBar、Sidebar、Layout 等组件间共享。

use crate::i18n::{Locale, t};

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

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Explorer => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>"#
            }
            Self::Search => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>"#
            }
            Self::SourceControl => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M6 9v12"/><path d="M18 9v12"/><path d="M12 15V3"/></svg>"#
            }
            Self::Extensions => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="9" height="9" rx="2"/><rect x="13" y="2" width="9" height="9" rx="2"/><rect x="13" y="13" width="9" height="9" rx="2"/><line x1="8" y1="21" x2="8" y2="12"/><line x1="8" y1="12" x2="3" y2="12"/><path d="M2.5 21h5.5a2 2 0 0 0 2-2v-5a2 2 0 0 0-2-2H2.5a.5.5 0 0 0-.5.5v8a.5.5 0 0 0 .5.5z"/></svg>"#
            }
        }
    }
}
