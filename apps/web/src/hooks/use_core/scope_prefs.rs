use crate::storage::prefs::{read_pref, remove_pref, write_pref};
use deve_core::models::PeerId;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};

use super::state::CoreSignals;

const SCOPE_PREF_KEY: &str = "deve.ui.last_scope";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredScopePref {
    repo_name: String,
    repo_id: String,
    #[serde(default)]
    active_branch: Option<String>,
}

pub(super) fn restore_scope_pref(signals: &CoreSignals) {
    let Some(raw) = read_pref(SCOPE_PREF_KEY) else {
        return;
    };
    let Some(scope) = parse_scope_pref(&raw) else {
        clear_scope_pref();
        return;
    };
    signals.set_current_repo.set(Some(scope.repo_name));
    signals.set_current_repo_id.set(Some(scope.repo_id));
    signals
        .set_active_branch
        .set(scope.active_branch.map(|peer_id| PeerId::new(&peer_id)));
}

pub(super) fn setup_scope_pref_effect(signals: &CoreSignals) {
    let signals = *signals;
    let last_saved = Rc::new(RefCell::new(read_pref(SCOPE_PREF_KEY)));
    Effect::new(move |_| {
        match next_scope_pref_json(
            signals.current_repo.get(),
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
    repo_name: Option<String>,
    repo_id: Option<String>,
    active_branch: Option<PeerId>,
    switching: bool,
) -> ScopePrefUpdate {
    if switching {
        return ScopePrefUpdate::Skip;
    }
    match (repo_name.filter(|name| !name.trim().is_empty()), repo_id) {
        (Some(repo_name), Some(repo_id)) if uuid::Uuid::parse_str(&repo_id).is_ok() => {
            ScopePrefUpdate::Persist(
                serde_json::to_string(&StoredScopePref {
                    repo_name,
                    repo_id,
                    active_branch: active_branch.map(|peer_id| peer_id.to_string()),
                })
                .expect("scope pref should serialize"),
            )
        }
        (None, None) if active_branch.is_none() => ScopePrefUpdate::Clear,
        _ => ScopePrefUpdate::Skip,
    }
}

fn parse_scope_pref(raw: &str) -> Option<StoredScopePref> {
    let scope: StoredScopePref = serde_json::from_str(raw).ok()?;
    (!scope.repo_name.trim().is_empty() && uuid::Uuid::parse_str(&scope.repo_id).is_ok())
        .then_some(scope)
}

enum ScopePrefUpdate {
    Persist(String),
    Clear,
    Skip,
}

#[cfg(test)]
#[path = "scope_prefs_test.rs"]
mod tests;
