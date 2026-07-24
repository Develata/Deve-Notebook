//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 07_network#remote-import-wire-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Exact connection and repository scope admission for Remote Import.

use super::{
    RemoteImportAvailability, RemoteImportClient, RemoteImportProjection, RemoteImportScope,
    SelectionState,
};
use crate::api::ConnectionStatus;
use deve_core::models::PeerId;
use deve_core::protocol::ScopeNonce;
use leptos::prelude::*;

impl RemoteImportClient {
    pub fn synchronize_scope(&self) -> RemoteImportAvailability {
        let status = self.ws.status.get();
        let connection_epoch = self.ws.connection_epoch.get();
        let repo_id = self.current_repo_id.get();
        let branch = self.active_branch.get();
        let scope_nonce = self.current_scope_nonce.get();
        let transitioning =
            self.pending_branch_switch.get().is_some() || self.pending_repo_switch.get().is_some();
        let (scope, availability) = self.scope_snapshot(
            status,
            connection_epoch,
            repo_id.as_deref(),
            branch,
            scope_nonce,
            transitioning,
            true,
        );
        self.synchronize_to(scope, availability);
        availability
    }

    fn current_scope(&self) -> Option<RemoteImportScope> {
        let repo_id = self.current_repo_id.get_untracked();
        self.scope_snapshot(
            self.ws.status.get_untracked(),
            self.ws.connection_epoch.get_untracked(),
            repo_id.as_deref(),
            self.active_branch.get_untracked(),
            self.current_scope_nonce.get_untracked(),
            self.pending_branch_switch.get_untracked().is_some()
                || self.pending_repo_switch.get_untracked().is_some(),
            false,
        )
        .0
    }

    pub(super) fn synchronize_current_scope(&self) -> Option<RemoteImportScope> {
        let current = self.current_scope();
        let availability = if current.is_some() {
            let repo_id = self.current_repo_id.get_untracked();
            RemoteImportAvailability::Ready {
                workspace_ingestion_blocked: self.ws.workspace_ingestion_blocked_for_untracked(
                    repo_id.as_deref(),
                    Some(self.current_scope_nonce.get_untracked()),
                ),
            }
        } else {
            self.untracked_unavailable_state()
        };
        self.synchronize_to(current, availability)
    }

    fn synchronize_to(
        &self,
        current: Option<RemoteImportScope>,
        availability: RemoteImportAvailability,
    ) -> Option<RemoteImportScope> {
        let changed = {
            let mut projected = self
                .projected_scope
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if *projected == current {
                false
            } else {
                *projected = current.clone();
                true
            }
        };
        if changed {
            self.pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clear();
            *self
                .selection
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = SelectionState::default();
            self.projection.set(RemoteImportProjection {
                availability,
                ..RemoteImportProjection::default()
            });
        } else {
            self.projection
                .update(|projection| projection.availability = availability);
        }
        current
    }

    #[allow(clippy::too_many_arguments)]
    fn scope_snapshot(
        &self,
        status: ConnectionStatus,
        connection_epoch: u64,
        repo_id: Option<&str>,
        branch: Option<PeerId>,
        scope_nonce: u64,
        transitioning: bool,
        tracked_ingestion: bool,
    ) -> (Option<RemoteImportScope>, RemoteImportAvailability) {
        if status != ConnectionStatus::Connected {
            return (None, RemoteImportAvailability::Offline);
        }
        if transitioning {
            return (None, RemoteImportAvailability::ScopeTransitioning);
        }
        let Some(repo_id_text) = repo_id else {
            return (None, RemoteImportAvailability::NoRepo);
        };
        let Ok(repo_id) = repo_id_text.parse() else {
            return (None, RemoteImportAvailability::NoRepo);
        };
        let workspace_ingestion_blocked = if tracked_ingestion {
            self.ws
                .workspace_ingestion_blocked_for(Some(repo_id_text), Some(scope_nonce))
        } else {
            self.ws
                .workspace_ingestion_blocked_for_untracked(Some(repo_id_text), Some(scope_nonce))
        };
        (
            Some(RemoteImportScope {
                connection_epoch,
                repo_id,
                branch,
                scope_nonce: ScopeNonce::new(scope_nonce),
            }),
            RemoteImportAvailability::Ready {
                workspace_ingestion_blocked,
            },
        )
    }

    fn untracked_unavailable_state(&self) -> RemoteImportAvailability {
        if self.ws.status.get_untracked() != ConnectionStatus::Connected {
            RemoteImportAvailability::Offline
        } else if self.pending_branch_switch.get_untracked().is_some()
            || self.pending_repo_switch.get_untracked().is_some()
        {
            RemoteImportAvailability::ScopeTransitioning
        } else {
            RemoteImportAvailability::NoRepo
        }
    }
}
