//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Trigger button for the repository switcher shell.

use crate::components::icons::ChevronRight;
use crate::i18n::{Locale, t};
use deve_core::models::RepoId;
use leptos::prelude::*;

use super::logic::{repo_switcher_after_trigger_click, repo_switcher_trigger_marker};

#[component]
pub(super) fn RepoSwitcherTrigger(
    show_menu: ReadSignal<bool>,
    set_show_menu: WriteSignal<bool>,
    set_action_repo: WriteSignal<Option<RepoId>>,
    set_renaming_repo: WriteSignal<Option<RepoId>>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            data-deve-repo-switcher-trigger=repo_switcher_trigger_marker()
            class="p-1 rounded text-secondary hover:bg-hover cursor-pointer transform transition-transform"
            class:rotate-90=move || show_menu.get()
            aria-expanded=move || show_menu.get()
            aria-haspopup="menu"
            on:click=move |e| {
                e.stop_propagation();
                set_show_menu.update(|v| *v = repo_switcher_after_trigger_click(*v));
                set_action_repo.set(None);
                set_renaming_repo.set(None);
            }
            title=move || t::sidebar::switch_repository(locale.get())
        >
            <ChevronRight />
        </button>
    }
}
