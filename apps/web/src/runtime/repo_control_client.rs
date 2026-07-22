//! plan_ref:
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract
//!
//! Typed Object Plane client for host-local aliases and repository lifecycle
//! jobs. It binds replies to the exact browser connection and repo scope, but
//! never infers lifecycle outcomes or owns repository authority.

use crate::api::WsService;
use deve_core::models::RepoId;
use deve_core::protocol::{
    ClientMessage, LocalRepoRemovalPreview, OpaqueFallbackBinding, RemovalConfirmationToken,
    RepoAliasBinding, RepoControlRequest, RepoControlResponse, RepoLifecycleIntent,
    RepoLifecycleOperation, RepoLifecycleOutcome, RepoLifecycleState, RepoListEntry,
    RepoRemovalFinalScope, ScopeNonce, ServerErrorCode, SwitchNonce,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mod accept;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoControlScope {
    connection_epoch: u64,
    repo_id: Option<RepoId>,
    branch: Option<String>,
    scope_nonce: u64,
}

impl RepoControlScope {
    pub fn new(
        connection_epoch: u64,
        repo_id: Option<RepoId>,
        branch: Option<String>,
        scope_nonce: u64,
    ) -> Self {
        Self {
            connection_epoch,
            repo_id,
            branch,
            scope_nonce,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingLifecycle {
    Create,
    #[allow(dead_code)] // R5 consumes the admitted removal job identity.
    Remove {
        repo_id: RepoId,
    },
}

impl PendingLifecycle {
    const fn operation(self) -> RepoLifecycleOperation {
        match self {
            Self::Create => RepoLifecycleOperation::Create,
            Self::Remove { .. } => RepoLifecycleOperation::Remove,
        }
    }

    fn accepts_target(self, target_repo_id: RepoId) -> bool {
        match self {
            Self::Create => true,
            Self::Remove { repo_id } => repo_id == target_repo_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingKind {
    Alias {
        repo_id: RepoId,
    },
    Lifecycle {
        lifecycle: PendingLifecycle,
        accepted: Option<(Uuid, RepoId)>,
    },
    RemovalPrepare {
        repo_id: RepoId,
        display_alias: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingRequest {
    scope: RepoControlScope,
    kind: PendingKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepoControlAdmission {
    AliasSet(RepoAliasBinding),
    LifecycleAccepted {
        request_id: Uuid,
        job_id: Uuid,
        target_repo_id: RepoId,
        operation: RepoLifecycleOperation,
    },
    RemovalPrepared {
        presentation: RepoRemovalPresentation,
    },
    RemovalFinalized {
        request_id: Option<Uuid>,
        job_id: Uuid,
        removed_repo_id: RepoId,
        final_repo_list: Vec<RepoListEntry>,
        scope: RepoRemovalFinalScope,
    },
    LifecycleStatus {
        request_id: Uuid,
        job_id: Uuid,
        target_repo_id: RepoId,
        operation: RepoLifecycleOperation,
        state: RepoLifecycleState,
        outcome: Option<RepoLifecycleOutcome>,
        publication_pending: bool,
    },
    Error {
        code: ServerErrorCode,
        lifecycle_request: bool,
        removal_request: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoRemovalPresentation {
    pub repo_id: RepoId,
    pub display_alias: String,
    pub preview: LocalRepoRemovalPreview,
    pub can_execute: bool,
}

#[derive(Clone)]
struct PreparedRemoval {
    scope: RepoControlScope,
    preparation_id: Uuid,
    repo_id: RepoId,
    confirmation_token: Option<RemovalConfirmationToken>,
    fallback_binding: Option<OpaqueFallbackBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedRemovalExecutionError {
    Missing,
    ScopeChanged,
    Blocked,
}

#[derive(Clone, Default)]
pub struct RepoControlClient {
    pending: Arc<Mutex<HashMap<Uuid, PendingRequest>>>,
    prepared_removal: Arc<Mutex<Option<PreparedRemoval>>>,
}

impl RepoControlClient {
    pub fn set_alias(
        &self,
        ws: &WsService,
        scope: RepoControlScope,
        repo_id: RepoId,
        alias: String,
        expected_alias_revision: u64,
    ) -> Uuid {
        let request_id = Uuid::new_v4();
        self.register(request_id, scope, PendingKind::Alias { repo_id });
        ws.send(ClientMessage::RepoControl(RepoControlRequest::SetAlias {
            request_id,
            repo_id,
            alias,
            expected_alias_revision,
        }));
        request_id
    }

    pub fn create_repo(
        &self,
        ws: &WsService,
        scope: RepoControlScope,
        initial_alias: String,
        switch_nonce: u64,
    ) -> Uuid {
        let scope_nonce = scope.scope_nonce;
        self.submit_lifecycle(
            ws,
            scope,
            PendingLifecycle::Create,
            RepoLifecycleIntent::Create {
                initial_alias,
                current_scope_nonce: ScopeNonce::new(scope_nonce),
                switch_nonce: SwitchNonce::new(switch_nonce),
            },
        )
    }

    pub fn prepare_remove_repo(
        &self,
        ws: &WsService,
        scope: RepoControlScope,
        repo_id: RepoId,
        display_alias: String,
    ) -> Uuid {
        let request_id = Uuid::new_v4();
        let scope_nonce = scope.scope_nonce;
        self.register(
            request_id,
            scope,
            PendingKind::RemovalPrepare {
                repo_id,
                display_alias,
            },
        );
        ws.send(ClientMessage::RepoControl(
            RepoControlRequest::PrepareLocalRepoRemoval {
                request_id,
                repo_id,
                current_scope_nonce: ScopeNonce::new(scope_nonce),
                fallback_repo_id: None,
            },
        ));
        request_id
    }

    pub fn execute_prepared_removal(
        &self,
        ws: &WsService,
        scope: RepoControlScope,
        repo_id: RepoId,
        switch_nonce: u64,
    ) -> Result<Uuid, PreparedRemovalExecutionError> {
        let prepared = {
            let mut slot = self
                .prepared_removal
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(prepared) = slot.as_ref() else {
                return Err(PreparedRemovalExecutionError::Missing);
            };
            if prepared.scope != scope || prepared.repo_id != repo_id {
                *slot = None;
                return Err(PreparedRemovalExecutionError::ScopeChanged);
            }
            if prepared.confirmation_token.is_none() {
                return Err(PreparedRemovalExecutionError::Blocked);
            }
            slot.take().expect("prepared removal checked above")
        };
        let request_id = Uuid::new_v4();
        self.register(
            request_id,
            scope.clone(),
            PendingKind::Lifecycle {
                lifecycle: PendingLifecycle::Remove { repo_id },
                accepted: None,
            },
        );
        ws.send(ClientMessage::RepoControl(
            RepoControlRequest::ExecuteLocalRepoRemoval {
                request_id,
                preparation_id: prepared.preparation_id,
                confirmation_token: prepared
                    .confirmation_token
                    .expect("prepared removal token checked above"),
                fallback_binding: prepared.fallback_binding,
                current_scope_nonce: ScopeNonce::new(scope.scope_nonce),
                switch_nonce: SwitchNonce::new(switch_nonce),
            },
        ));
        Ok(request_id)
    }

    pub fn cancel_prepared_removal(&self, scope: &RepoControlScope, repo_id: RepoId) -> bool {
        let mut slot = self
            .prepared_removal
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot
            .as_ref()
            .is_some_and(|prepared| prepared.scope == *scope && prepared.repo_id == repo_id)
        {
            *slot = None;
            true
        } else {
            false
        }
    }

    pub fn synchronize_scope(&self, current_scope: &RepoControlScope) -> bool {
        let mut slot = self
            .prepared_removal
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot
            .as_ref()
            .is_some_and(|prepared| prepared.scope != *current_scope)
        {
            *slot = None;
            true
        } else {
            false
        }
    }

    /// Rebind transport-local observation after an exact connection or scope
    /// epoch change. The backend remains the job owner; the client only asks
    /// for typed state and never derives the lifecycle outcome.
    pub fn resume_lifecycles(&self, ws: &WsService, current_scope: RepoControlScope) -> usize {
        let request_ids = {
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pending
                .iter_mut()
                .filter_map(|(request_id, request)| {
                    let PendingKind::Lifecycle { .. } = &request.kind else {
                        return None;
                    };
                    if request.scope == current_scope {
                        return None;
                    }
                    request.scope = current_scope.clone();
                    Some(*request_id)
                })
                .collect::<Vec<_>>()
        };
        for request_id in &request_ids {
            ws.send(ClientMessage::RepoControl(
                RepoControlRequest::GetLifecycle {
                    request_id: *request_id,
                },
            ));
        }
        request_ids.len()
    }

    fn submit_lifecycle(
        &self,
        ws: &WsService,
        scope: RepoControlScope,
        lifecycle: PendingLifecycle,
        lifecycle_intent: RepoLifecycleIntent,
    ) -> Uuid {
        let request_id = Uuid::new_v4();
        self.register(
            request_id,
            scope,
            PendingKind::Lifecycle {
                lifecycle,
                accepted: None,
            },
        );
        ws.send(ClientMessage::RepoControl(
            RepoControlRequest::SubmitLifecycle {
                request_id,
                lifecycle_intent,
            },
        ));
        request_id
    }

    fn register(&self, request_id: Uuid, scope: RepoControlScope, kind: PendingKind) {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(request_id, PendingRequest { scope, kind });
    }
}

#[cfg(test)]
mod tests;
