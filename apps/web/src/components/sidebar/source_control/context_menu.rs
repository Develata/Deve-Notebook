// apps/web/src/components/sidebar/source_control/context_menu.rs
//! # Source Control Context Menu
//!
//! Dropdown menu to toggle section visibility (repos, changes, graph).

use crate::components::icons::*;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn SectionMenu(
    show_menu: RwSignal<bool>,
    show_repos: RwSignal<bool>,
    show_changes: RwSignal<bool>,
    show_graph: RwSignal<bool>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    move || {
        if !show_menu.get() {
            return view! {}.into_any();
        }
        view! {
            <div
                class="absolute right-0 top-full mt-1 w-32 bg-panel border border-default shadow-lg rounded z-50 text-[12px] py-1"
                on:click=move |e| e.stop_propagation()
            >
                <MenuItem label=move || t::source_control::repositories(locale.get()) checked=show_repos />
                <MenuItem label=move || t::source_control::changes(locale.get()) checked=show_changes />
                <MenuItem label=move || t::source_control::graph(locale.get()) checked=show_graph />
            </div>
        }
        .into_any()
    }
}

#[component]
fn MenuItem(
    label: impl Fn() -> &'static str + Send + 'static,
    checked: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div
            class="px-3 py-1.5 hover:bg-hover cursor-pointer flex items-center justify-between"
            on:click=move |_| checked.update(|v| *v = !*v)
        >
            <span>{label}</span>
            {move || {
                if checked.get() {
                    view! { <Check class="w-3 h-3" /> }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}
