//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract
//!   - 09_web_thin_client_ledger#document-create-intent
//!
//! 目录创建逻辑。

use super::checked_existing_is_dir;
use super::create::{
    DocumentCreateReceipt, DocumentCreateResult, classify_execution_error, storage_conflict,
};
use crate::server::AppState;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::repo_scope::ResolvedRepo;
use deve_core::ledger::StructureCreateIdentityState;
use deve_core::models::NodeId;
use deve_core::protocol::{DocumentCreateProjectionOutcome, DocumentRecoveryScope};
use std::sync::Arc;

pub async fn handle_folder_create(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
    proposed_node_id: NodeId,
) -> DocumentCreateResult {
    let gate = state.repo_mutation_gate();
    let admission = match gate.admit_mounted_repo(scope.repo_id) {
        Ok(admission) => admission,
        Err(error) => return Err(error.server_error()),
    };
    let folder_path = filename.trim_end_matches('/');
    let execution = gate
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let scope =
                match crate::server::repo_mutation::revalidate_writable_resolved_repo(state, scope)
                {
                    Ok(scope) => scope,
                    Err(error) => return MutationExecution::not_committed(error),
                };
            let publication =
                MutationPublication::document_recovery(scope.repo_id, DocumentRecoveryScope::None);
            match inspect_identity(state, &scope, folder_path, proposed_node_id) {
                Ok(StructureCreateIdentityState::Exact { .. }) => {
                    if path.is_dir() {
                        MutationExecution::committed(
                            DocumentCreateProjectionOutcome::Written,
                            publication,
                        )
                    } else {
                        match state
                            .sync_manager
                            .rebuild_projection_local_repo(&scope.repo_name)
                        {
                            Ok(_) => MutationExecution::committed(
                                DocumentCreateProjectionOutcome::Written,
                                publication,
                            ),
                            Err(error) => MutationExecution::projection_degraded(
                                DocumentCreateProjectionOutcome::RecoveryRequired,
                                error,
                                publication,
                            ),
                        }
                    }
                }
                Ok(StructureCreateIdentityState::Conflict) => {
                    MutationExecution::not_committed(storage_conflict())
                }
                Ok(StructureCreateIdentityState::Vacant) => {
                    match checked_existing_is_dir(path, "folder create target revalidation") {
                        Ok(Some(false)) => {
                            return MutationExecution::not_committed(storage_conflict());
                        }
                        Ok(Some(true)) | Ok(None) => {}
                        Err(error) => return MutationExecution::not_committed(error),
                    }
                    let (_node_id, _ops) =
                        match state.repo.apply_dir_create_structure_with_id_in_local_repo(
                            &scope.repo_name,
                            folder_path,
                            Some(proposed_node_id),
                            "local_create",
                        ) {
                            Ok(value) => value,
                            Err(error) => {
                                return match inspect_identity(
                                    state,
                                    &scope,
                                    folder_path,
                                    proposed_node_id,
                                ) {
                                    Ok(StructureCreateIdentityState::Exact { .. }) => {
                                        MutationExecution::committed_partial(error, publication)
                                    }
                                    _ => MutationExecution::not_committed(error),
                                };
                            }
                        };
                    match state
                        .sync_manager
                        .rebuild_projection_local_repo(&scope.repo_name)
                    {
                        Ok(_) => MutationExecution::committed(
                            DocumentCreateProjectionOutcome::Written,
                            publication,
                        ),
                        Err(error) => MutationExecution::projection_degraded(
                            DocumentCreateProjectionOutcome::RecoveryRequired,
                            error,
                            publication,
                        ),
                    }
                }
                Err(error) => MutationExecution::not_committed(error),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { value, .. }) => Ok(DocumentCreateReceipt {
            node_id: proposed_node_id,
            doc_id: None,
            projection_outcome: value,
        }),
        Ok(MutationExecution::ProjectionDegraded { error, .. })
        | Ok(MutationExecution::CommittedPartial { error, .. }) => {
            tracing::error!(error = ?error, "Folder Create committed but projection recovery is required");
            Ok(DocumentCreateReceipt {
                node_id: proposed_node_id,
                doc_id: None,
                projection_outcome: DocumentCreateProjectionOutcome::RecoveryRequired,
            })
        }
        Ok(MutationExecution::NotCommitted(error)) => {
            tracing::error!(error = ?error, "Folder Create rejected before authority commit");
            Err(classify_execution_error(error))
        }
        Err(error) => Err(error.server_error()),
    }
}

fn inspect_identity(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    folder_path: &str,
    proposed_node_id: NodeId,
) -> anyhow::Result<StructureCreateIdentityState> {
    state.repo.inspect_structure_create_identity_in_local_repo(
        &scope.repo_name,
        folder_path,
        proposed_node_id,
        None,
    )
}
