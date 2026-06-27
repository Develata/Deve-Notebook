//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Pure state helpers for the repository switcher view.

use crate::i18n::{Locale, t};
use deve_core::models::RepoId;
use deve_core::protocol::RepoListEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RepoSwitcherRow {
    pub repo_id: Option<RepoId>,
    pub name: String,
    pub execution_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RepoSwitcherSwitchTarget {
    pub selector_name: String,
    pub expected_name: String,
    pub repo_id: Option<RepoId>,
}

impl RepoSwitcherRow {
    pub(super) fn key(&self) -> String {
        self.repo_id
            .map(|repo_id| repo_id.to_string())
            .unwrap_or_else(|| format!("legacy:{}", self.name))
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

pub(super) fn repo_switcher_rows(
    repos: Vec<String>,
    entries: Vec<RepoListEntry>,
) -> Vec<RepoSwitcherRow> {
    if !entries.is_empty() {
        return entries
            .into_iter()
            .map(|entry| RepoSwitcherRow {
                repo_id: Some(entry.repo_id),
                name: entry.name,
                execution_name: entry.execution_name,
            })
            .collect();
    }

    repos
        .into_iter()
        .map(|name| RepoSwitcherRow {
            repo_id: None,
            execution_name: name.clone(),
            name,
        })
        .collect()
}

pub(super) fn repo_switcher_row_is_active(
    current_repo: Option<String>,
    current_repo_id: Option<String>,
    row: &RepoSwitcherRow,
) -> bool {
    if let Some(repo_id) = row.repo_id
        && let Some(current_repo_id) = current_repo_id
    {
        return current_repo_id == repo_id.to_string();
    }
    current_repo.as_deref() == Some(row.name.as_str())
}

pub(super) fn repo_switcher_switch_target(row: &RepoSwitcherRow) -> RepoSwitcherSwitchTarget {
    RepoSwitcherSwitchTarget {
        selector_name: row.execution_name.clone(),
        expected_name: row.name.clone(),
        repo_id: row.repo_id,
    }
}

pub(super) fn repo_switcher_remove_confirmed(locale: Locale, name: &str) -> bool {
    let message = t::sidebar::remove_repository_confirm(locale, name);
    web_sys::window()
        .and_then(|window| window.confirm_with_message(&message).ok())
        .unwrap_or(false)
}
