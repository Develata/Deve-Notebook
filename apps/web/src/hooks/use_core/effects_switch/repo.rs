//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use leptos::prelude::{GetUntracked, Set};

use super::{RepoSwitchOutcome, RepoSwitchSignals};

pub fn handle_repo_switched(
    name: String,
    uuid: String,
    switch_nonce: Option<u64>,
    signals: RepoSwitchSignals,
) -> RepoSwitchOutcome {
    let current_repo = signals.current_repo.get_untracked();
    let current_repo_id = signals.current_repo_id.get_untracked();
    let pending_nonce = signals.pending_repo_switch_nonce.get_untracked();
    let current_scope_nonce = signals.current_scope_nonce.get_untracked();
    match signals.pending_repo_switch.get_untracked() {
        Some(pending) if pending == name => {
            if pending_nonce != switch_nonce {
                leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
                return ignored_repo_switch();
            }
            signals.set_pending_repo_switch.set(None);
            signals.set_pending_repo_switch_nonce.set(None);
            if let Some(switch_nonce) = switch_nonce {
                signals.set_current_scope_nonce.set(switch_nonce);
            }
        }
        Some(_) => {
            leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
            return ignored_repo_switch();
        }
        None => {
            let rebinding_after_branch_switch = current_repo.is_none()
                && current_repo_id.is_none()
                && switch_nonce == Some(current_scope_nonce);
            let same_repo = current_repo.as_deref() == Some(name.as_str())
                && current_repo_id.as_deref() == Some(uuid.as_str());
            let newer_same_repo_scope =
                same_repo && switch_nonce.is_some_and(|nonce| nonce >= current_scope_nonce);
            if !rebinding_after_branch_switch
                && !newer_same_repo_scope
                && (pending_nonce != switch_nonce || !same_repo)
            {
                leptos::logging::warn!("忽略无 pending 的 RepoSwitched: {}", name);
                return ignored_repo_switch();
            }
            signals.set_pending_repo_switch_nonce.set(None);
            if let Some(switch_nonce) = switch_nonce {
                signals.set_current_scope_nonce.set(switch_nonce);
            }
        }
    }

    let same_repo = !uuid.is_empty() && current_repo_id.as_deref() == Some(uuid.as_str());
    let scope_changed = switch_nonce.is_some() && switch_nonce != Some(current_scope_nonce);
    signals.set_current_repo.set(Some(name));
    signals
        .set_current_repo_id
        .set((!uuid.is_empty()).then_some(uuid));
    if !same_repo {
        signals.set_current_doc.set(None);
    }
    RepoSwitchOutcome {
        accepted: true,
        should_refresh: scope_changed || !same_repo,
    }
}

fn ignored_repo_switch() -> RepoSwitchOutcome {
    RepoSwitchOutcome {
        accepted: false,
        should_refresh: false,
    }
}
