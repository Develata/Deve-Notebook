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
    ClientMessage, RepoAliasBinding, RepoControlRequest, RepoControlResponse, RepoLifecycleIntent,
    RepoLifecycleOperation, RepoLifecycleOutcome, RepoLifecycleState, ScopeNonce, ServerErrorCode,
    SwitchNonce,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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
    Remove { repo_id: RepoId },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingKind {
    Alias {
        repo_id: RepoId,
    },
    Lifecycle {
        lifecycle: PendingLifecycle,
        accepted: Option<(Uuid, RepoId)>,
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
    },
}

#[derive(Clone, Default)]
pub struct RepoControlClient {
    pending: Arc<Mutex<HashMap<Uuid, PendingRequest>>>,
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

    pub fn remove_repo(
        &self,
        ws: &WsService,
        scope: RepoControlScope,
        repo_id: RepoId,
        switch_nonce: u64,
    ) -> Uuid {
        let scope_nonce = scope.scope_nonce;
        self.submit_lifecycle(
            ws,
            scope,
            PendingLifecycle::Remove { repo_id },
            RepoLifecycleIntent::Remove {
                repo_id,
                current_scope_nonce: ScopeNonce::new(scope_nonce),
                switch_nonce: SwitchNonce::new(switch_nonce),
            },
        )
    }

    /// Rebind transport-local observation after an exact browser reconnect.
    /// The backend remains the job owner; the client only asks for typed state.
    pub fn resume_lifecycles(&self, ws: &WsService, current_scope: RepoControlScope) -> usize {
        let request_ids = {
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pending
                .iter_mut()
                .filter_map(|(request_id, request)| {
                    let PendingKind::Lifecycle { .. } = request.kind else {
                        return None;
                    };
                    if request.scope.connection_epoch == current_scope.connection_epoch {
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

    pub fn accept(
        &self,
        response: RepoControlResponse,
        current_scope: &RepoControlScope,
    ) -> Option<RepoControlAdmission> {
        let request_id = response_request_id(&response);
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let request = pending.get_mut(&request_id)?;
        if request.scope != *current_scope {
            pending.remove(&request_id);
            return None;
        }

        match response {
            RepoControlResponse::AliasSet { binding, .. } => {
                let PendingKind::Alias { repo_id } = request.kind else {
                    pending.remove(&request_id);
                    return None;
                };
                pending.remove(&request_id);
                (binding.repo_id == repo_id).then_some(RepoControlAdmission::AliasSet(binding))
            }
            RepoControlResponse::LifecycleAccepted {
                job_id,
                target_repo_id,
                ..
            } => {
                let PendingKind::Lifecycle {
                    lifecycle,
                    accepted,
                } = &mut request.kind
                else {
                    pending.remove(&request_id);
                    return None;
                };
                if !lifecycle.accepts_target(target_repo_id) {
                    pending.remove(&request_id);
                    return None;
                }
                *accepted = Some((job_id, target_repo_id));
                Some(RepoControlAdmission::LifecycleAccepted {
                    request_id,
                    job_id,
                    target_repo_id,
                    operation: lifecycle.operation(),
                })
            }
            RepoControlResponse::LifecycleStatus {
                job_id,
                target_repo_id,
                operation,
                state,
                outcome,
                publication_pending,
                ..
            } => {
                let PendingKind::Lifecycle {
                    lifecycle,
                    accepted,
                } = &mut request.kind
                else {
                    pending.remove(&request_id);
                    return None;
                };
                if lifecycle.operation() != operation || !lifecycle.accepts_target(target_repo_id) {
                    pending.remove(&request_id);
                    return None;
                }
                match accepted {
                    Some(identity) if *identity != (job_id, target_repo_id) => {
                        pending.remove(&request_id);
                        return None;
                    }
                    Some(_) => {}
                    None => *accepted = Some((job_id, target_repo_id)),
                }
                if state == RepoLifecycleState::Terminal && !publication_pending {
                    pending.remove(&request_id);
                }
                Some(RepoControlAdmission::LifecycleStatus {
                    request_id,
                    job_id,
                    target_repo_id,
                    operation,
                    state,
                    outcome,
                    publication_pending,
                })
            }
            RepoControlResponse::Error { error, .. } => {
                let lifecycle_request = matches!(request.kind, PendingKind::Lifecycle { .. });
                pending.remove(&request_id);
                Some(RepoControlAdmission::Error {
                    code: error.code,
                    lifecycle_request,
                })
            }
        }
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

fn response_request_id(response: &RepoControlResponse) -> Uuid {
    match response {
        RepoControlResponse::AliasSet { request_id, .. }
        | RepoControlResponse::LifecycleAccepted { request_id, .. }
        | RepoControlResponse::LifecycleStatus { request_id, .. }
        | RepoControlResponse::Error { request_id, .. } => *request_id,
    }
}

#[cfg(test)]
mod tests;
