//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Atomic thin-client projection of the backend-produced removal finalization.

use crate::api::WsService;
use crate::hooks::use_core::scope_prefs::clear_scope_pref;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::RepoId;
use deve_core::protocol::{RepoListEntry, RepoRemovalFinalScope};
use leptos::prelude::{GetUntracked, Set, Update};

use super::message_control_runtime_repo::clear_repo_scoped_runtime;

pub(super) fn apply(
    ws: &WsService,
    signals: CoreSignals,
    removed_repo_id: RepoId,
    final_repo_list: Vec<RepoListEntry>,
    scope: RepoRemovalFinalScope,
) -> bool {
    let current_repo_id = signals
        .current_repo_id
        .get_untracked()
        .and_then(|value| value.parse::<RepoId>().ok());
    let current_scope_nonce = signals.current_scope_nonce.get_untracked();
    let next_scope_nonce = scope.scope_nonce().get();
    let aliases = final_repo_list
        .iter()
        .map(|entry| entry.display_alias.clone())
        .collect::<Vec<_>>();

    match scope {
        RepoRemovalFinalScope::RepoBound {
            repo_id,
            scope_nonce,
        } if current_repo_id != Some(removed_repo_id)
            && current_repo_id == Some(repo_id)
            && scope_nonce.get() == current_scope_nonce =>
        {
            signals.set_repo_list.set(aliases);
            signals.set_repo_entries.set(final_repo_list);
            signals.set_pending_repo_switch.set(None);
            signals.set_removal_preview.set(None);
            true
        }
        RepoRemovalFinalScope::RepoBound {
            repo_id,
            scope_nonce,
        } if current_repo_id == Some(removed_repo_id)
            && scope_nonce.get() > current_scope_nonce =>
        {
            let Some(binding) = final_repo_list
                .iter()
                .find(|entry| entry.repo_id == repo_id)
            else {
                return false;
            };
            clear_scope_pref();
            ws.clear_writer_ready();
            signals.set_handshake_ready.set(false);
            signals.set_active_branch.set(None);
            signals.set_current_doc.set(None);
            signals.set_docs.set(Vec::new());
            signals.set_tree_nodes.set(Vec::new());
            clear_repo_scoped_runtime(signals);
            signals
                .set_current_repo
                .set(Some(binding.display_alias.clone()));
            signals.set_current_repo_id.set(Some(repo_id.to_string()));
            signals.set_current_scope_nonce.set(scope_nonce.get());
            signals.set_repo_list.set(aliases);
            signals.set_repo_entries.set(final_repo_list);
            signals.set_explicit_repo_selection_required.set(false);
            signals.set_removal_preview.set(None);
            signals
                .set_handshake_retry_nonce
                .update(|nonce| *nonce = nonce.wrapping_add(1));
            true
        }
        RepoRemovalFinalScope::NoScope { scope_nonce }
            if current_repo_id == Some(removed_repo_id)
                && scope_nonce.get() > current_scope_nonce =>
        {
            clear_scope_pref();
            ws.clear_writer_ready();
            signals.set_handshake_ready.set(false);
            signals.set_active_branch.set(None);
            signals.set_current_repo.set(None);
            signals.set_current_repo_id.set(None);
            signals.set_current_doc.set(None);
            signals.set_docs.set(Vec::new());
            signals.set_tree_nodes.set(Vec::new());
            clear_repo_scoped_runtime(signals);
            signals.set_current_scope_nonce.set(next_scope_nonce);
            signals.set_repo_list.set(aliases);
            signals.set_repo_entries.set(final_repo_list);
            signals.set_explicit_repo_selection_required.set(true);
            signals.set_removal_preview.set(None);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::state::init_signals;
    use deve_core::protocol::{RepoReadiness, ScopeNonce};
    use leptos::prelude::{GetUntracked, Owner, Set, signal};

    fn entry(repo_id: RepoId, alias: &str) -> RepoListEntry {
        RepoListEntry {
            repo_id,
            display_alias: alias.into(),
            alias_revision: 1,
            readiness: RepoReadiness::Mounted,
        }
    }

    fn harness(repo_id: RepoId, nonce: u64) -> (Owner, WsService, CoreSignals) {
        let owner = Owner::new();
        owner.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let signals = init_signals(signal(ConnectionStatus::Connected).0);
        signals.set_current_repo.set(Some("current".into()));
        signals.set_current_repo_id.set(Some(repo_id.to_string()));
        signals.set_current_scope_nonce.set(nonce);
        (owner, ws, signals)
    }

    #[test]
    fn current_repo_removal_commits_backend_no_scope() {
        let removed = RepoId::new_v4();
        let (_owner, ws, signals) = harness(removed, 7);

        assert!(apply(
            &ws,
            signals,
            removed,
            Vec::new(),
            RepoRemovalFinalScope::NoScope {
                scope_nonce: ScopeNonce::new(8),
            },
        ));
        assert!(signals.current_repo_id.get_untracked().is_none());
        assert_eq!(signals.current_scope_nonce.get_untracked(), 8);
        assert!(signals.explicit_repo_selection_required.get_untracked());
    }

    #[test]
    fn current_repo_removal_commits_backend_fallback() {
        let removed = RepoId::new_v4();
        let fallback = RepoId::new_v4();
        let (_owner, ws, signals) = harness(removed, 7);

        assert!(apply(
            &ws,
            signals,
            removed,
            vec![entry(fallback, "fallback")],
            RepoRemovalFinalScope::RepoBound {
                repo_id: fallback,
                scope_nonce: ScopeNonce::new(8),
            },
        ));
        assert_eq!(
            signals.current_repo_id.get_untracked().as_deref(),
            Some(fallback.to_string().as_str())
        );
        assert_eq!(signals.current_scope_nonce.get_untracked(), 8);
    }

    #[test]
    fn non_current_removal_only_projects_backend_list() {
        let current = RepoId::new_v4();
        let removed = RepoId::new_v4();
        let (_owner, ws, signals) = harness(current, 7);

        assert!(apply(
            &ws,
            signals,
            removed,
            vec![entry(current, "current")],
            RepoRemovalFinalScope::RepoBound {
                repo_id: current,
                scope_nonce: ScopeNonce::new(7),
            },
        ));
        assert_eq!(
            signals.current_repo_id.get_untracked(),
            Some(current.to_string())
        );
        assert_eq!(signals.current_scope_nonce.get_untracked(), 7);
    }

    #[test]
    fn stale_backend_scope_is_rejected_without_projection_change() {
        let removed = RepoId::new_v4();
        let (_owner, ws, signals) = harness(removed, 7);

        assert!(!apply(
            &ws,
            signals,
            removed,
            Vec::new(),
            RepoRemovalFinalScope::NoScope {
                scope_nonce: ScopeNonce::new(7),
            },
        ));
        assert_eq!(
            signals.current_repo_id.get_untracked(),
            Some(removed.to_string())
        );
        assert_eq!(signals.current_scope_nonce.get_untracked(), 7);
    }
}
