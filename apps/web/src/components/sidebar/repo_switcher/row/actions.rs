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
use leptos::ev::MouseEvent;
use leptos::prelude::*;

use super::super::logic::{repo_switcher_remove_button_marker, repo_switcher_rename_button_marker};

#[component]
pub(super) fn RepoSwitcherRowActions(
    repo_id: RepoId,
    action_repo: ReadSignal<Option<RepoId>>,
    set_action_repo: WriteSignal<Option<RepoId>>,
    set_renaming_repo: WriteSignal<Option<RepoId>>,
    set_show_menu: WriteSignal<bool>,
    rename_value_name: String,
    set_rename_name: WriteSignal<String>,
    remove_current_name: String,
    on_remove_repo: Callback<RepoRemoveRequest>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        {move || {
            if action_repo.get() != Some(repo_id) {
                return view! {}.into_any();
            }
            let rename_value_name = rename_value_name.clone();
            let remove_current_name = remove_current_name.clone();
            view! {
                <div class="absolute right-2 top-8 z-[calc(var(--z-floating)_+_2)] w-36 bg-panel border border-default shadow-lg rounded-md py-1 text-xs">
                    <button
                        type="button"
                        class="flex min-h-[44px] w-full items-center gap-2 px-2 py-2 text-left hover:bg-hover"
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
                        class="flex min-h-[44px] w-full items-center gap-2 px-2 py-2 text-left text-danger hover:bg-hover"
                        data-deve-repo-switcher-remove=repo_switcher_remove_button_marker()
                        on:click=move |e: MouseEvent| {
                            e.stop_propagation();
                            on_remove_repo.run(RepoRemoveRequest {
                                repo_id,
                                current_name: remove_current_name.clone(),
                            });
                            set_action_repo.set(None);
                            set_renaming_repo.set(None);
                            set_show_menu.set(false);
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
