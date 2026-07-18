//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Persists accepted document edits and emits ack/broadcast messages.

use crate::server::{AppState, channel::DualChannel, repo_scope::ResolvedRepo};
use deve_core::models::{DocId, FactActor, Op};
use deve_core::protocol::{ClientOrigin, ConfirmedOp, ServerError, ServerErrorCode};
use std::sync::Arc;

use super::edit_checks::reject_missing_doc;
use super::edit_checks::{ExistingClientOpCheck, confirm_existing_client_op};
use super::write_confirmation::{WriteConfirmation, emit_write_confirmation};
use crate::server::repo_mutation::{MutationExecution, MutationPublication};

pub(super) struct ClientEditAppend<'a> {
    pub(super) state: &'a Arc<AppState>,
    pub(super) scope: &'a ResolvedRepo,
    pub(super) ch: &'a DualChannel,
    pub(super) scope_nonce: u64,
    pub(super) doc_id: DocId,
    pub(super) op: Op,
    pub(super) client_id: u64,
    pub(super) client_op_id: u64,
}

pub(super) async fn append_client_edit(input: ClientEditAppend<'_>) {
    let repo_id = input.scope.repo_id;

    let ClientEditAppend {
        state,
        scope,
        ch,
        scope_nonce,
        doc_id,
        op,
        client_id,
        client_op_id,
    } = input;
    let gate = state.repo_mutation_gate();
    let execution = gate
        .execute_mounted_repo(repo_id, &state.tx, || {
            let repo_name = match crate::server::repo_mutation::revalidate_writable_local_repo(
                state,
                repo_id,
                &scope.repo_name,
            ) {
                Ok(repo_name) => repo_name,
                Err(error) => {
                    return MutationExecution::not_committed(Some(ServerError::with_detail(
                        ServerErrorCode::StoragePersistFailed,
                        error.to_string(),
                    )));
                }
            };
            let bound_scope = ResolvedRepo {
                repo_id,
                repo_name: repo_name.clone(),
                session_name: scope.session_name.clone(),
                branch: None,
            };
            if let Err(error) = reject_missing_doc(state, &repo_name, doc_id) {
                return MutationExecution::not_committed(Some(error));
            }
            if let Err(error) = state
                .repo
                .repair_client_op_index_if_missing_in_local_repo(&repo_name)
            {
                return MutationExecution::not_committed(Some(ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    error.to_string(),
                )));
            }
            if confirm_existing_client_op(ExistingClientOpCheck {
                state,
                scope: &bound_scope,
                ch,
                scope_nonce,
                doc_id,
                op: &op,
                client_id,
                client_op_id,
            }) {
                return MutationExecution::not_committed(None);
            }
            let writer = state
                .repo
                .local_fact_writer(FactActor::new("browser_edit").expect("static actor is valid"));
            match writer.append_client_content_in_local_repo(
                &repo_name,
                doc_id,
                op.clone(),
                chrono::Utc::now().timestamp_millis(),
                client_id,
                client_op_id,
            ) {
                Ok((global_seq, _local_seq)) => {
                    let entry = ConfirmedOp::new(
                        global_seq,
                        op.clone(),
                        Some(ClientOrigin {
                            client_id,
                            client_op_id,
                        }),
                    );
                    let writeback = state
                        .sync_manager
                        .persist_doc_in_local_repo(&repo_name, doc_id);
                    let degraded_recovery = writeback.as_ref().err().map(|_| {
                        match MutationPublication::document_recovery(
                            repo_id,
                            deve_core::protocol::DocumentRecoveryScope::Exact(vec![doc_id]),
                        ) {
                            MutationPublication::ProjectionRecovery(recovery) => recovery,
                            _ => unreachable!("document recovery constructor is stable"),
                        }
                    });
                    let publication = MutationPublication::ConfirmedEdit {
                        repo_id,
                        branch: None,
                        scope_nonce: Some(scope_nonce),
                        doc_id,
                        entry,
                        recovery: degraded_recovery,
                    };
                    match writeback {
                        Ok(()) => MutationExecution::committed(global_seq, publication),
                        Err(err) => {
                            tracing::error!(
                                doc_id = %doc_id,
                                client_op_id,
                                "Workspace projection writeback failed after ledger commit: {:?}",
                                err
                            );
                            MutationExecution::projection_degraded(
                                global_seq,
                                Some(ServerError::with_detail(
                                    ServerErrorCode::StoragePersistFailed,
                                    format!(
                                        "Projection writeback failed after ledger commit: {err}"
                                    ),
                                )),
                                publication,
                            )
                        }
                    }
                }
                Err(err) => {
                    if confirm_existing_client_op(ExistingClientOpCheck {
                        state,
                        scope: &bound_scope,
                        ch,
                        scope_nonce,
                        doc_id,
                        op: &op,
                        client_id,
                        client_op_id,
                    }) {
                        return MutationExecution::not_committed(None);
                    }
                    tracing::error!("Failed to persist op: {:?}", err);
                    MutationExecution::not_committed(Some(ServerError::with_detail(
                        ServerErrorCode::StoragePersistFailed,
                        err.to_string(),
                    )))
                }
            }
        })
        .await;

    emit_write_confirmation(
        WriteConfirmation {
            ch,
            scope,
            scope_nonce,
            doc_id,
            client_op_id,
        },
        execution,
    );
}
