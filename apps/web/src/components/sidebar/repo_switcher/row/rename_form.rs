//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Rename form for a repository switcher row.

use crate::components::icons::{Check, X};
use crate::i18n::{Locale, t};
use crate::runtime::domain::RepoRenameRequest;
use deve_core::models::RepoId;
use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;

use super::super::logic::{
    repo_switcher_can_submit_rename_repo, repo_switcher_rename_input_marker,
};

#[component]
pub(super) fn RepoSwitcherRenameForm(
    repo_id: Option<RepoId>,
    current_name: String,
    rename_name: ReadSignal<String>,
    set_rename_name: WriteSignal<String>,
    set_renaming_repo: WriteSignal<Option<RepoId>>,
    set_action_repo: WriteSignal<Option<RepoId>>,
    set_show_menu: WriteSignal<bool>,
    on_rename_repo: Callback<RepoRenameRequest>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let current_name_for_submit = current_name.clone();
    let current_name_for_disabled = current_name;

    view! {
        <form
            class="px-2 py-1.5 flex items-center gap-1 bg-accent-subtle"
            on:submit=move |ev: SubmitEvent| {
                ev.prevent_default();
                let Some(repo_id) = repo_id else {
                    return;
                };
                let new_name = rename_name.get_untracked();
                if repo_switcher_can_submit_rename_repo(&current_name_for_submit, &new_name) {
                    on_rename_repo.run(RepoRenameRequest {
                        repo_id,
                        current_name: current_name_for_submit.clone(),
                        new_name: new_name.trim().to_string(),
                    });
                    set_rename_name.set(String::new());
                    set_renaming_repo.set(None);
                    set_action_repo.set(None);
                    set_show_menu.set(false);
                }
            }
        >
            <input
                type="text"
                class="min-w-0 flex-1 bg-input border border-default rounded px-2 py-1 text-xs outline-none focus:border-accent"
                data-deve-repo-switcher-rename-input=repo_switcher_rename_input_marker()
                prop:value=move || rename_name.get()
                aria-label=move || t::sidebar::rename_repository(locale.get())
                on:input=move |ev| set_rename_name.set(event_target_value(&ev))
            />
            <button
                type="submit"
                class="p-1 rounded text-secondary hover:text-accent hover:bg-hover disabled:opacity-40"
                disabled=move || {
                    !repo_switcher_can_submit_rename_repo(
                        &current_name_for_disabled,
                        &rename_name.get(),
                    )
                }
                title=move || t::common::confirm(locale.get())
                aria-label=move || t::common::confirm(locale.get())
            >
                <Check />
            </button>
            <button
                type="button"
                class="p-1 rounded text-secondary hover:text-danger hover:bg-hover"
                title=move || t::common::cancel(locale.get())
                aria-label=move || t::common::cancel(locale.get())
                on:click=move |e: MouseEvent| {
                    e.stop_propagation();
                    set_rename_name.set(String::new());
                    set_renaming_repo.set(None);
                }
            >
                <X />
            </button>
        </form>
    }
}
