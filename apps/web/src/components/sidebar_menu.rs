// apps\web\src\components
//! # SidebarMenu 组件 (SidebarMenu Component)
//!
//! 文件树上下文菜单，提供重命名、复制、移动、删除等操作。

#![allow(dead_code)] // danger/with_separator: 菜单项 Builder 模式预留

use leptos::prelude::*;

use crate::components::dropdown::{Align, AnchorRect, Dropdown};
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
    label: &'static str,
    icon: &'static str, // SVG path
    is_danger: bool,
    is_separator_before: bool,
}

impl MenuItem {
    const fn new(action: MenuAction, label: &'static str, icon: &'static str) -> Self {
        Self {
            action,
            label,
            icon,
            is_danger: false,
            is_separator_before: false,
        }
    }

    const fn danger(mut self) -> Self {
        self.is_danger = true;
        self
    }

    const fn with_separator(mut self) -> Self {
        self.is_separator_before = true;
        self
    }
}

/// 定义所有菜单项
const MENU_ITEMS: &[MenuItem] = &[
    MenuItem::new(
        MenuAction::Rename,
        "Rename",
        "M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10",
    ),
    MenuItem::new(
        MenuAction::Copy,
        "Copy",
        "M15.75 17.25v3.375c0 .621-.504 1.125-1.125 1.125h-9.75a1.125 1.125 0 01-1.125-1.125V7.875c0-.621.504-1.125 1.125-1.125H6.75a9.06 9.06 0 011.5.124m7.5 10.376h3.375c.621 0 1.125-.504 1.125-1.125V11.25c0-4.46-3.243-8.161-7.5-8.876a9.06 9.06 0 00-1.5-.124H9.375c-.621 0-1.125.504-1.125 1.125v3.5m7.5 10.375H9.375a1.125 1.125 0 01-1.125-1.125v-9.25m12 6.625v-1.875a3.375 3.375 0 00-3.375-3.375h-1.5a1.125 1.125 0 01-1.125-1.125v-1.5a3.375 3.375 0 00-3.375-3.375H9.75",
    ),
    MenuItem::new(
        MenuAction::OpenInNewWindow,
        "Open in New Window",
        "M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25",
    ),
    MenuItem {
        action: MenuAction::MoveTo,
        label: "Move to...",
        icon: "M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15M12 9l-3 3m0 0l3 3m-3-3h12.75",
        is_danger: false,
        is_separator_before: true,
    },
    MenuItem {
        action: MenuAction::Delete,
        label: "Delete",
        icon: "M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0",
        is_danger: true,
        is_separator_before: true,
    },
];

#[component]
pub fn SidebarMenu(
    #[prop(into)] on_action: Callback<MenuAction>,
    #[prop(into)] on_close: Callback<()>,
    anchor: ReadSignal<Option<AnchorRect>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <Dropdown anchor=anchor.into() on_close=on_close align=Align::Right offset=6.0>
            <div class="w-48 bg-panel rounded-md shadow-lg border border-default py-1 text-sm text-primary select-none animate-in fade-in zoom-in-95 duration-100 ease-out origin-top-right">
                {MENU_ITEMS.iter().map(|item| {
                    let action = item.action;
                    let icon_path = item.icon;
                    let is_danger = item.is_danger;
                    let has_separator = item.is_separator_before;

                    view! {
                        <>
                            {if has_separator {
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
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    class=format!(
                                        "w-4 h-4 {}",
                                        if is_danger { "text-red-500 group-hover:text-red-600" } else { "text-muted" }
                                    )
                                >
                                    <path stroke-linecap="round" stroke-linejoin="round" d={icon_path} />
                                </svg>
                                {move || action.label(locale.get())}
                            </button>
                        </>
                    }
                }).collect_view()}
            </div>
        </Dropdown>
    }
}
