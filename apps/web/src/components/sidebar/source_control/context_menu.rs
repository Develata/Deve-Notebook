// apps/web/src/components/sidebar/source_control/context_menu.rs
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! # Source Control Context Menu
//!
//! Dropdown menu to toggle section visibility (repos, changes, graph).

use crate::components::icons::*;
use crate::i18n::{Locale, t};
use leptos::ev::MouseEvent;
use leptos::prelude::*;

const SECTION_MENU_REPOSITORIES: &str = "repositories";
const SECTION_MENU_CHANGES: &str = "changes";
const SECTION_MENU_GRAPH: &str = "graph";
const SECTION_MENU_HISTORY: &str = "history";

#[cfg(test)]
fn section_menu_item_ids() -> [&'static str; 4] {
    [
        SECTION_MENU_REPOSITORIES,
        SECTION_MENU_CHANGES,
        SECTION_MENU_GRAPH,
        SECTION_MENU_HISTORY,
    ]
}

#[component]
pub fn SectionMenu(
    show_menu: RwSignal<bool>,
    show_repos: RwSignal<bool>,
    show_changes: RwSignal<bool>,
    show_graph: RwSignal<bool>,
    show_history: RwSignal<bool>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    move || {
        if !show_menu.get() {
            return view! {}.into_any();
        }
        view! {
            <>
                <div
                    class="fixed inset-0 z-[var(--z-floating)]"
                    data-deve-sc-section-menu-backdrop="true"
                    on:click=move |e: MouseEvent| {
                        e.stop_propagation();
                        show_menu.set(false);
                    }
                ></div>
                <div
                    id="source-control-section-menu"
                    role="menu"
                    data-deve-sc-section-menu="true"
                    class="absolute right-0 top-full mt-1 w-32 bg-panel border border-default shadow-lg rounded z-[calc(var(--z-floating)_+_1)] text-[12px] py-1"
                    on:click=move |e: MouseEvent| e.stop_propagation()
                >
                    <MenuItem
                        item_id=SECTION_MENU_REPOSITORIES
                        label=move || t::source_control::repositories(locale.get())
                        checked=show_repos
                        show_menu=show_menu
                    />
                    <MenuItem
                        item_id=SECTION_MENU_CHANGES
                        label=move || t::source_control::changes(locale.get())
                        checked=show_changes
                        show_menu=show_menu
                    />
                    <MenuItem
                        item_id=SECTION_MENU_GRAPH
                        label=move || t::source_control::graph(locale.get())
                        checked=show_graph
                        show_menu=show_menu
                    />
                    <MenuItem
                        item_id=SECTION_MENU_HISTORY
                        label=move || t::source_control::history(locale.get())
                        checked=show_history
                        show_menu=show_menu
                    />
                </div>
            </>
        }
        .into_any()
    }
}

#[component]
fn MenuItem(
    item_id: &'static str,
    label: impl Fn() -> &'static str + Send + 'static,
    checked: RwSignal<bool>,
    show_menu: RwSignal<bool>,
) -> impl IntoView {
    let on_click = move |e: MouseEvent| {
        e.stop_propagation();
        checked.update(|v| *v = !*v);
        show_menu.set(false);
    };

    view! {
        <button
            type="button"
            role="menuitemcheckbox"
            data-deve-sc-section-menu-item=item_id
            aria-checked=move || checked.get().to_string()
            class="w-full px-3 py-1.5 hover:bg-hover text-left flex items-center justify-between focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/40"
            on:click=on_click
        >
            <span>{label}</span>
            {move || {
                if checked.get() {
                    view! { <Check class="w-3 h-3" /> }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::section_menu_item_ids;

    #[test]
    fn source_control_history_visibility_menu_item_present() {
        assert_eq!(
            section_menu_item_ids(),
            ["repositories", "changes", "graph", "history"]
        );
    }
}
