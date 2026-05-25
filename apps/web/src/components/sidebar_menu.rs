// apps/web/src/components/sidebar_menu.rs
//! plan_ref:
//!   - 11_ui_design_01_web#web-layout-persistence
//!
//! # SidebarMenu 组件 (SidebarMenu Component)
//!
//! 文件树上下文菜单，提供重命名、复制、移动、删除等操作。

use leptos::prelude::*;

use crate::components::dropdown::{Align, AnchorRect, Dropdown};
use crate::i18n::{Locale, t};

mod item;

use item::SidebarMenuItems;

/// 菜单操作类型
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAction {
    Rename,
    Copy,
    OpenInNewWindow,
    MoveTo,
    Delete,
}

impl MenuAction {
    pub fn label(&self, locale: Locale) -> &'static str {
        match self {
            Self::Rename => t::context_menu::rename(locale),
            Self::Copy => t::context_menu::copy(locale),
            Self::OpenInNewWindow => t::context_menu::open_in_new_window(locale),
            Self::MoveTo => t::context_menu::move_to(locale),
            Self::Delete => t::context_menu::delete(locale),
        }
    }
}

#[component]
pub fn SidebarMenu(
    is_readonly: Signal<bool>,
    #[prop(into)] on_action: Callback<MenuAction>,
    #[prop(into)] on_close: Callback<()>,
    anchor: ReadSignal<Option<AnchorRect>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <Dropdown anchor=anchor.into() on_close=on_close align=Align::Right offset=6.0>
            <div class="w-48 bg-panel rounded-md shadow-lg border border-default py-1 text-sm text-primary select-none animate-in fade-in zoom-in-95 duration-100 ease-out origin-top-right">
                <SidebarMenuItems locale is_readonly on_action on_close />
            </div>
        </Dropdown>
    }
}
