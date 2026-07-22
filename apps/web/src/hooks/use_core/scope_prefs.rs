//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!   - 04_repository#repo-scope-runtime
//!
use crate::storage::prefs::{read_pref, remove_pref, write_pref};
use deve_core::models::{PeerId, RepoId};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

use super::state::CoreSignals;

const SCOPE_PREF_KEY: &str = "deve.ui.last_scope";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredScopePref {
    repo_id: RepoId,
    branch: StoredScopeBranchKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StoredScopeBranchKind {
    Local,
}

pub(super) fn restore_scope_pref(signals: &CoreSignals) {
    let Some(raw) = read_pref(SCOPE_PREF_KEY) else {
        return;
    };
    let Some(scope) = parse_scope_pref(&raw) else {
        clear_scope_pref();
        return;
    };
    signals.set_current_repo.set(None);
    signals
        .set_current_repo_id
        .set(Some(scope.repo_id.to_string()));
}

pub(super) fn setup_scope_pref_effect(signals: &CoreSignals) {
    let signals = *signals;
    let last_saved = Rc::new(RefCell::new(read_pref(SCOPE_PREF_KEY)));
    Effect::new(move |_| {
        match next_scope_pref_json(
            signals.current_repo_id.get(),
            signals.active_branch.get(),
            signals.pending_repo_switch.get().is_some()
                || signals.pending_branch_switch.get().is_some(),
        ) {
            ScopePrefUpdate::Persist(json) => {
                if last_saved.borrow().as_deref() == Some(json.as_str()) {
                    return;
                }
                if write_pref(SCOPE_PREF_KEY, &json).is_ok() {
                    *last_saved.borrow_mut() = Some(json);
                }
            }
            ScopePrefUpdate::Clear => {
                if last_saved.borrow().is_none() {
                    return;
                }
                clear_scope_pref();
                last_saved.borrow_mut().take();
            }
            ScopePrefUpdate::Skip => {}
        }
    });
}

pub(super) fn clear_scope_pref() {
    remove_pref(SCOPE_PREF_KEY);
}

fn next_scope_pref_json(
    repo_id: Option<String>,
    active_branch: Option<PeerId>,
    switching: bool,
) -> ScopePrefUpdate {
    if switching {
        return ScopePrefUpdate::Skip;
    }
    if active_branch.is_some() {
        return ScopePrefUpdate::Clear;
    }
    match repo_id.and_then(|value| value.parse::<RepoId>().ok()) {
        Some(repo_id) => {
            serialize_scope_pref(repo_id).map_or(ScopePrefUpdate::Skip, ScopePrefUpdate::Persist)
        }
        None => ScopePrefUpdate::Clear,
    }
}

fn serialize_scope_pref(repo_id: RepoId) -> Option<String> {
    match serde_json::to_string(&StoredScopePref {
        repo_id,
        branch: StoredScopeBranchKind::Local,
    }) {
        Ok(json) => Some(json),
        Err(err) => {
            leptos::logging::warn!("无法序列化 repo scope preference: {}", err);
            None
        }
    }
}

fn parse_scope_pref(raw: &str) -> Option<StoredScopePref> {
    serde_json::from_str(raw).ok()
}

enum ScopePrefUpdate {
    Persist(String),
    Clear,
    Skip,
}

#[cfg(test)]
mod tests;
