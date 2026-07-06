//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Action menu for repository switcher rows.

use crate::components::icons::{Pencil, Trash2};
use crate::i18n::{Locale, t};
use crate::runtime::domain::RepoRemoveRequest;
use deve_core::models::RepoId;
use deve_core::protocol::RepoListEntry;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

use super::super::logic::{
    repo_switcher_remove_button_marker, repo_switcher_remove_confirmed,
    repo_switcher_rename_button_marker,
};

#[component]
pub(super) fn RepoSwitcherRowActions(
    repo_id: Option<RepoId>,
    repo_entries: ReadSignal<Vec<RepoListEntry>>,
    action_repo: ReadSignal<Option<RepoId>>,
    set_action_repo: WriteSignal<Option<RepoId>>,
    set_renaming_repo: WriteSignal<Option<RepoId>>,
    set_show_menu: WriteSignal<bool>,
    rename_value_name: String,
    set_rename_name: WriteSignal<String>,
    action_title_name: String,
    remove_current_name: String,
    on_remove_repo: Callback<RepoRemoveRequest>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        {move || {
            let Some(repo_id) = repo_id else {
                return view! {}.into_any();
            };
            if action_repo.get() != Some(repo_id) {
                return view! {}.into_any();
            }
            let fallback_name = repo_entries
                .get()
                .into_iter()
                .find(|entry| entry.repo_id != repo_id)
                .map(|entry| entry.name);
            let rename_value_name = rename_value_name.clone();
            let action_title_name = action_title_name.clone();
            let remove_current_name = remove_current_name.clone();
            let fallback_name_for_remove = fallback_name.clone();
            view! {
                <div class="absolute right-2 top-8 z-[calc(var(--z-floating)_+_2)] w-36 bg-panel border border-default shadow-lg rounded-md py-1 text-xs">
                    <button
                        type="button"
                        class="w-full px-2 py-1.5 flex items-center gap-2 text-left hover:bg-hover"
                        data-deve-repo-switcher-rename=repo_switcher_rename_button_marker()
                        on:click=move |e: MouseEvent| {
                            e.stop_propagation();
                            set_rename_name.set(rename_value_name.clone());
                            set_renaming_repo.set(Some(repo_id));
                            set_action_repo.set(None);
                        }
                    >
                        <Pencil class="w-3.5 h-3.5" />
                        <span class="truncate">{move || t::sidebar::rename_repository(locale.get())}</span>
                    </button>
                    <button
                        type="button"
                        class="w-full px-2 py-1.5 flex items-center gap-2 text-left text-danger hover:bg-hover"
                        data-deve-repo-switcher-remove=repo_switcher_remove_button_marker()
                        on:click=move |e: MouseEvent| {
                            e.stop_propagation();
                            if repo_switcher_remove_confirmed(locale.get(), &action_title_name) {
                                on_remove_repo.run(RepoRemoveRequest {
                                    repo_id,
                                    current_name: remove_current_name.clone(),
                                    fallback_name: fallback_name_for_remove.clone(),
                                });
                                set_action_repo.set(None);
                                set_renaming_repo.set(None);
                                set_show_menu.set(false);
                            }
                        }
                    >
                        <Trash2 class="w-3.5 h-3.5" />
                        <span class="truncate">{move || t::sidebar::remove_repository(locale.get())}</span>
                    </button>
                </div>
            }.into_any()
        }}
    }
}
