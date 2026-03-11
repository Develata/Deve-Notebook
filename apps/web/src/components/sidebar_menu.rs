// apps/web/src/components/sidebar_menu.rs
//! # SidebarMenu 组件 (SidebarMenu Component)
//!
//! 文件树上下文菜单，提供重命名、复制、移动、删除等操作。

use leptos::prelude::*;

use crate::components::dropdown::{Align, AnchorRect, Dropdown};
use crate::components::icons;
use crate::i18n::{Locale, t};

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

/// 菜单项配置
struct MenuItem {
    action: MenuAction,
    is_danger: bool,
    separator_before: bool,
}

const MENU_ITEMS: &[MenuItem] = &[
    MenuItem {
        action: MenuAction::Rename,
        is_danger: false,
        separator_before: false,
    },
    MenuItem {
        action: MenuAction::Copy,
        is_danger: false,
        separator_before: false,
    },
    MenuItem {
        action: MenuAction::OpenInNewWindow,
        is_danger: false,
        separator_before: false,
    },
    MenuItem {
        action: MenuAction::MoveTo,
        is_danger: false,
        separator_before: true,
    },
    MenuItem {
        action: MenuAction::Delete,
        is_danger: true,
        separator_before: true,
    },
];

/// 根据 MenuAction 返回对应 Lucide 图标组件
fn menu_icon(action: MenuAction, class: &str) -> AnyView {
    let cls = class.to_string();
    match action {
        MenuAction::Rename => view! { <icons::Pencil class=cls/> }.into_any(),
        MenuAction::Copy => view! { <icons::Copy class=cls/> }.into_any(),
        MenuAction::OpenInNewWindow => view! { <icons::ExternalLink class=cls/> }.into_any(),
        MenuAction::MoveTo => view! { <icons::FolderInput class=cls/> }.into_any(),
        MenuAction::Delete => view! { <icons::Trash2 class=cls/> }.into_any(),
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
                {MENU_ITEMS.iter().filter(|item| {
                    !is_readonly.get()
                        || matches!(
                            item.action,
                            MenuAction::OpenInNewWindow
                        )
                }).map(|item| {
                    let action = item.action;
                    let is_danger = item.is_danger;
                    let has_sep = item.separator_before;
                    let icon_cls = format!(
                        "w-4 h-4 {}",
                        if is_danger { "text-red-500 group-hover:text-red-600" } else { "text-muted" }
                    );

                    view! {
                        <>
                            {if has_sep {
                                Some(view! { <div class="my-1 border-t border-default"></div> })
                            } else {
                                None
                            }}
                            <button
                                class=format!(
                                    "w-full text-left px-3 py-1.5 hover:bg-hover flex items-center gap-2 {}",
                                    if is_danger { "text-red-600 group" } else { "" }
                                )
                                on:click=move |_| {
                                    leptos::logging::log!("SidebarMenu: Button clicked, action={:?}", action);
                                    on_action.run(action);
                                    on_close.run(());
                                }
                            >
                                {menu_icon(action, &icon_cls)}
                                {move || action.label(locale.get())}
                            </button>
                        </>
                    }
                }).collect_view()}
            </div>
        </Dropdown>
    }
}
