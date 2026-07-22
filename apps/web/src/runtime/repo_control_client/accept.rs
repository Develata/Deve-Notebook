//! plan_ref:
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract
//!
//! Exact response admission. This module projects backend outcomes only; it
//! owns no lifecycle transition or repository authority.

use super::*;

impl RepoControlClient {
    pub fn accept(
        &self,
        response: RepoControlResponse,
        current_scope: &RepoControlScope,
    ) -> Option<RepoControlAdmission> {
        if let RepoControlResponse::LocalRepoRemovalObserverInvalidated {
            job_id,
            removed_repo_id,
            final_repo_list,
            scope,
        } = &response
        {
            let RepoRemovalFinalScope::NoScope { scope_nonce } = scope else {
                return None;
            };
            if current_scope.repo_id != Some(*removed_repo_id)
                || scope_nonce.get() <= current_scope.scope_nonce
            {
                return None;
            }
            self.synchronize_scope(&RepoControlScope::new(
                current_scope.connection_epoch,
                None,
                None,
                scope_nonce.get(),
            ));
            return Some(RepoControlAdmission::RemovalFinalized {
                request_id: None,
                job_id: *job_id,
                removed_repo_id: *removed_repo_id,
                final_repo_list: final_repo_list.clone(),
                scope: RepoRemovalFinalScope::NoScope {
                    scope_nonce: *scope_nonce,
                },
            });
        }
        let request_id = response_request_id(&response)?;
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
                let PendingKind::Alias { repo_id } = request.kind.clone() else {
                    pending.remove(&request_id);
                    return None;
                };
                pending.remove(&request_id);
                (binding.repo_id == repo_id).then_some(RepoControlAdmission::AliasSet(binding))
            }
            RepoControlResponse::LocalRepoRemovalPrepared {
                preparation_id,
                repo_id,
                preview,
                confirmation_token,
                fallback_binding,
                ..
            } => {
                let PendingKind::RemovalPrepare {
                    repo_id: expected_repo_id,
                    display_alias,
                } = request.kind.clone()
                else {
                    pending.remove(&request_id);
                    return None;
                };
                pending.remove(&request_id);
                if repo_id != expected_repo_id {
                    return None;
                }
                let can_execute = confirmation_token.is_some();
                *self
                    .prepared_removal
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(PreparedRemoval {
                    scope: current_scope.clone(),
                    preparation_id,
                    repo_id,
                    confirmation_token,
                    fallback_binding,
                });
                Some(RepoControlAdmission::RemovalPrepared {
                    presentation: RepoRemovalPresentation {
                        repo_id,
                        display_alias,
                        preview,
                        can_execute,
                    },
                })
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
                    let waits_for_finalization = lifecycle.operation()
                        == RepoLifecycleOperation::Remove
                        && matches!(
                            outcome,
                            Some(
                                RepoLifecycleOutcome::Succeeded
                                    | RepoLifecycleOutcome::CommittedPartial
                            )
                        );
                    if !waits_for_finalization {
                        pending.remove(&request_id);
                    }
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
            RepoControlResponse::LocalRepoRemovalSettled {
                job_id,
                removed_repo_id,
                final_repo_list,
                scope,
                ..
            } => {
                let PendingKind::Lifecycle {
                    lifecycle: PendingLifecycle::Remove { repo_id },
                    accepted,
                } = request.kind.clone()
                else {
                    pending.remove(&request_id);
                    return None;
                };
                if repo_id != removed_repo_id || accepted != Some((job_id, removed_repo_id)) {
                    pending.remove(&request_id);
                    return None;
                }
                pending.remove(&request_id);
                Some(RepoControlAdmission::RemovalFinalized {
                    request_id: Some(request_id),
                    job_id,
                    removed_repo_id,
                    final_repo_list,
                    scope,
                })
            }
            RepoControlResponse::LocalRepoRemovalObserverInvalidated { .. } => unreachable!(),
            RepoControlResponse::Error { error, .. } => {
                let lifecycle_request = matches!(&request.kind, PendingKind::Lifecycle { .. });
                let removal_request = matches!(
                    &request.kind,
                    PendingKind::RemovalPrepare { .. }
                        | PendingKind::Lifecycle {
                            lifecycle: PendingLifecycle::Remove { .. },
                            ..
                        }
                );
                pending.remove(&request_id);
                Some(RepoControlAdmission::Error {
                    code: error.code,
                    lifecycle_request,
                    removal_request,
                })
            }
        }
    }
}

fn response_request_id(response: &RepoControlResponse) -> Option<Uuid> {
    match response {
        RepoControlResponse::AliasSet { request_id, .. }
        | RepoControlResponse::LocalRepoRemovalPrepared { request_id, .. }
        | RepoControlResponse::LifecycleAccepted { request_id, .. }
        | RepoControlResponse::LifecycleStatus { request_id, .. }
        | RepoControlResponse::LocalRepoRemovalSettled { request_id, .. }
        | RepoControlResponse::Error { request_id, .. } => Some(*request_id),
        RepoControlResponse::LocalRepoRemovalObserverInvalidated { .. } => None,
    }
}
