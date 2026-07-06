//! plan_ref:
//!   - 04_repository#repo-selector-resolution-contract
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Row composition for the repository switcher menu.

mod actions;
mod display;
mod rename_form;

use crate::i18n::Locale;
use crate::runtime::domain::{RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest};
use deve_core::models::RepoId;
use deve_core::protocol::RepoListEntry;
use leptos::prelude::*;

use self::actions::RepoSwitcherRowActions;
use self::display::RepoSwitcherDisplayRow;
use self::rename_form::RepoSwitcherRenameForm;
use super::logic::{RepoSwitcherRow, repo_switcher_row_is_renaming};

#[component]
pub(super) fn RepoSwitcherRowView(
    row: RepoSwitcherRow,
    current_repo: ReadSignal<Option<String>>,
    current_repo_id: ReadSignal<Option<String>>,
    repo_entries: ReadSignal<Vec<RepoListEntry>>,
    on_switch_repo: Callback<RepoSwitchRequest>,
    on_rename_repo: Callback<RepoRenameRequest>,
    on_remove_repo: Callback<RepoRemoveRequest>,
    set_show_menu: WriteSignal<bool>,
    set_show_create: WriteSignal<bool>,
    action_repo: ReadSignal<Option<RepoId>>,
    set_action_repo: WriteSignal<Option<RepoId>>,
    renaming_repo: ReadSignal<Option<RepoId>>,
    set_renaming_repo: WriteSignal<Option<RepoId>>,
    rename_name: ReadSignal<String>,
    set_rename_name: WriteSignal<String>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let row_id = row.repo_id;
    let rename_current_name = row.name.clone();
    let remove_current_name = row.name.clone();
    let action_title_name = row.name.clone();
    let rename_value_name = row.name.clone();

    view! {
        <div class="relative">
            {move || if repo_switcher_row_is_renaming(renaming_repo.get(), row_id) {
                view! {
                    <RepoSwitcherRenameForm
                        repo_id=row_id
                        current_name=rename_current_name.clone()
                        rename_name=rename_name
                        set_rename_name=set_rename_name
                        set_renaming_repo=set_renaming_repo
                        set_action_repo=set_action_repo
                        set_show_menu=set_show_menu
                        on_rename_repo=on_rename_repo.clone()
                        locale=locale
                    />
                }.into_any()
            } else {
                view! {
                    <RepoSwitcherDisplayRow
                        row=row.clone()
                        current_repo=current_repo
                        current_repo_id=current_repo_id
                        on_switch_repo=on_switch_repo.clone()
                        set_show_menu=set_show_menu
                        set_show_create=set_show_create
                        set_action_repo=set_action_repo
                        set_renaming_repo=set_renaming_repo
                        locale=locale
                    />
                }.into_any()
            }}
            <RepoSwitcherRowActions
                repo_id=row_id
                repo_entries=repo_entries
                action_repo=action_repo
                set_action_repo=set_action_repo
                set_renaming_repo=set_renaming_repo
                set_show_menu=set_show_menu
                rename_value_name=rename_value_name
                set_rename_name=set_rename_name
                action_title_name=action_title_name
                remove_current_name=remove_current_name
                on_remove_repo=on_remove_repo
                locale=locale
            />
        </div>
    }
}
