//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Pure state helpers for the repository switcher view.

use deve_core::models::RepoId;
use deve_core::protocol::RepoListEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RepoSwitcherRow {
    pub repo_id: RepoId,
    pub name: String,
    pub alias_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RepoSwitcherSwitchTarget {
    pub expected_name: String,
    pub repo_id: RepoId,
}

impl RepoSwitcherRow {
    pub(super) fn key(&self) -> String {
        self.repo_id.to_string()
    }
}

pub(super) fn repo_switcher_trigger_marker() -> &'static str {
    "repo-switcher-trigger"
}

pub(super) fn repo_switcher_menu_marker(open: bool) -> Option<&'static str> {
    open.then_some("visible")
}

pub(super) fn repo_switcher_backdrop_marker() -> &'static str {
    "repo-switcher-outside"
}

pub(super) fn repo_switcher_item_marker() -> &'static str {
    "repo-switcher-item"
}

pub(super) fn repo_switcher_after_trigger_click(open: bool) -> bool {
    !open
}

pub(super) fn repo_switcher_after_outside_click() -> bool {
    false
}

pub(super) fn repo_switcher_after_item_click() -> bool {
    false
}

pub(super) fn repo_switcher_should_reset_transient_state(menu_open: bool) -> bool {
    !menu_open
}

pub(super) fn repo_switcher_create_button_marker() -> &'static str {
    "repo-switcher-create"
}

pub(super) fn repo_switcher_create_input_marker() -> &'static str {
    "repo-switcher-create-input"
}

pub(super) fn repo_switcher_action_button_marker() -> &'static str {
    "repo-switcher-actions"
}

pub(super) fn repo_switcher_rename_button_marker() -> &'static str {
    "repo-switcher-rename"
}

pub(super) fn repo_switcher_remove_button_marker() -> &'static str {
    "repo-switcher-remove"
}

pub(super) fn repo_switcher_rename_input_marker() -> &'static str {
    "repo-switcher-rename-input"
}

pub(super) fn repo_switcher_can_submit_create_repo(name: &str) -> bool {
    !name.trim().is_empty()
}

pub(super) fn repo_switcher_can_submit_rename_repo(current_name: &str, next_name: &str) -> bool {
    let next_name = next_name.trim();
    !next_name.is_empty() && next_name != current_name
}

pub(super) fn repo_switcher_row_is_renaming(
    renaming_repo: Option<RepoId>,
    row_repo_id: RepoId,
) -> bool {
    renaming_repo == Some(row_repo_id)
}

pub(super) fn repo_switcher_rows(entries: Vec<RepoListEntry>) -> Vec<RepoSwitcherRow> {
    entries
        .into_iter()
        .map(|entry| RepoSwitcherRow {
            repo_id: entry.repo_id,
            name: entry.display_alias,
            alias_revision: entry.alias_revision,
        })
        .collect()
}

pub(super) fn repo_switcher_row_is_active(
    current_repo_id: Option<String>,
    row: &RepoSwitcherRow,
) -> bool {
    current_repo_id.as_deref() == Some(row.repo_id.to_string().as_str())
}

pub(super) fn repo_switcher_switch_target(row: &RepoSwitcherRow) -> RepoSwitcherSwitchTarget {
    RepoSwitcherSwitchTarget {
        expected_name: row.name.clone(),
        repo_id: row.repo_id,
    }
}
