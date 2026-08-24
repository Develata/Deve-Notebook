//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract
//!   - 09_web_thin_client_ledger#document-create-intent
//!
//! 文件创建逻辑。

use super::checked_exists;
use super::create::{
    DocumentCreateReceipt, DocumentCreateResult, classify_execution_error, storage_conflict,
};
use super::file_register::create_file_from_content;
use crate::server::AppState;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::repo_scope::ResolvedRepo;
use deve_core::ledger::StructureCreateIdentityState;
use deve_core::models::{DocId, NodeId};
use deve_core::protocol::{DocumentCreateProjectionOutcome, DocumentRecoveryScope};
use std::sync::Arc;

pub async fn handle_file_create(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
    proposed_node_id: NodeId,
) -> DocumentCreateResult {
    let proposed_doc_id = DocId(proposed_node_id.0);
    let gate = state.repo_mutation_gate();
    let admission = match gate.admit_mounted_repo(scope.repo_id) {
        Ok(admission) => admission,
        Err(error) => return Err(error.server_error()),
    };

    let execution = gate
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let scope =
                match crate::server::repo_mutation::revalidate_writable_resolved_repo(state, scope)
                {
                    Ok(scope) => scope,
                    Err(error) => return MutationExecution::not_committed(error),
                };
            let publication = MutationPublication::document_recovery(
                scope.repo_id,
                DocumentRecoveryScope::Exact(vec![proposed_doc_id]),
            );
            match inspect_identity(state, &scope, filename, proposed_node_id, proposed_doc_id) {
                Ok(StructureCreateIdentityState::Exact { .. }) => {
                    if path.is_file() {
                        MutationExecution::committed(
                            DocumentCreateProjectionOutcome::Written,
                            publication,
                        )
                    } else {
                        match state
                            .sync_manager
                            .rebuild_projection_local_repo(&scope.repo_name)
                        {
                            Ok(()) => MutationExecution::committed(
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
                    match checked_exists(path, "create target") {
                        Ok(true) => {
                            return MutationExecution::not_committed(storage_conflict());
                        }
                        Ok(false) => {}
                        Err(error) => return MutationExecution::not_committed(error),
                    }
                    match create_file_from_content(
                        state,
                        &scope,
                        filename,
                        "",
                        "local_create",
                        Some(proposed_doc_id),
                    ) {
                        Ok((doc_id, _ops)) if doc_id == proposed_doc_id => {
                            MutationExecution::committed(
                                DocumentCreateProjectionOutcome::Written,
                                publication,
                            )
                        }
                        Ok((doc_id, _)) => MutationExecution::not_committed(anyhow::anyhow!(
                            "Document Create returned unexpected doc identity: {doc_id}"
                        )),
                        Err(error) => match inspect_identity(
                            state,
                            &scope,
                            filename,
                            proposed_node_id,
                            proposed_doc_id,
                        ) {
                            Ok(StructureCreateIdentityState::Exact { .. }) => {
                                MutationExecution::committed_partial(error, publication)
                            }
                            _ => MutationExecution::not_committed(error),
                        },
                    }
                }
                Err(error) => MutationExecution::not_committed(error),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { value, .. }) => Ok(DocumentCreateReceipt {
            node_id: proposed_node_id,
            doc_id: Some(proposed_doc_id),
            projection_outcome: value,
        }),
        Ok(MutationExecution::ProjectionDegraded { error, .. })
        | Ok(MutationExecution::CommittedPartial { error, .. }) => {
            tracing::error!(error = ?error, "Document Create committed but projection recovery is required");
            Ok(DocumentCreateReceipt {
                node_id: proposed_node_id,
                doc_id: Some(proposed_doc_id),
                projection_outcome: DocumentCreateProjectionOutcome::RecoveryRequired,
            })
        }
        Ok(MutationExecution::NotCommitted(error)) => {
            tracing::error!(error = ?error, "Document Create rejected before authority commit");
            Err(classify_execution_error(error))
        }
        Err(error) => Err(error.server_error()),
    }
}

fn inspect_identity(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    filename: &str,
    proposed_node_id: NodeId,
    proposed_doc_id: DocId,
) -> anyhow::Result<StructureCreateIdentityState> {
    state.repo.inspect_structure_create_identity_in_local_repo(
        &scope.repo_name,
        filename,
        proposed_node_id,
        Some(proposed_doc_id),
    )
}
