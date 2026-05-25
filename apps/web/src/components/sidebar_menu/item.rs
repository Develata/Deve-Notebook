//! plan_ref:
//!   - 11_ui_design_01_web#web-layout-persistence
//!
use super::MenuAction;
use crate::components::icons;
use crate::i18n::Locale;
use leptos::prelude::*;

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

#[component]
pub(super) fn SidebarMenuItems(
    locale: RwSignal<Locale>,
    is_readonly: Signal<bool>,
    on_action: Callback<MenuAction>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        {move || MENU_ITEMS
            .iter()
            .filter(|item| !is_readonly.get() || matches!(item.action, MenuAction::OpenInNewWindow))
            .map(|item| {
                let action = item.action;
                let is_danger = item.is_danger;
                let has_sep = item.separator_before;
                let icon_cls = format!(
                    "w-4 h-4 {}",
                    if is_danger {
                        "text-red-500 group-hover:text-red-600"
                    } else {
                        "text-muted"
                    }
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
            })
            .collect_view()}
    }
}

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
