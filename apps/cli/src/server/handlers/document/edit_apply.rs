//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Persists accepted document edits and emits ack/broadcast messages.

use crate::server::{AppState, channel::DualChannel, repo_scope::ResolvedRepo};
use deve_core::models::{DocId, FactActor, Op};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::edit_checks::{ExistingClientOpCheck, confirm_existing_client_op};
use super::edit_support::reject_edit;
use super::write_confirmation::{CommitOutcome, CommittedWrite, emit_commit_outcome};
use super::write_gate::with_repo_write_gate;

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

pub(super) fn append_client_edit(input: ClientEditAppend<'_>) {
    let ch = input.ch;
    let scope_nonce = input.scope_nonce;
    let doc_id = input.doc_id;
    let client_op_id = input.client_op_id;
    let repo_id = input.scope.repo_id;

    if let Err(error) = with_repo_write_gate(repo_id, || append_client_edit_locked(input)) {
        reject_edit(ch, scope_nonce, doc_id, client_op_id, error);
    }
}

fn append_client_edit_locked(input: ClientEditAppend<'_>) {
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
    let writer = state
        .repo
        .local_fact_writer(FactActor::new("browser_edit").expect("static actor is valid"));
    match writer.append_client_content_in_local_repo(
        &scope.repo_name,
        doc_id,
        op.clone(),
        chrono::Utc::now().timestamp_millis(),
        client_id,
        client_op_id,
    ) {
        Ok((global_seq, _local_seq)) => {
            let outcome = match state
                .sync_manager
                .persist_doc_in_local_repo(&scope.repo_name, doc_id)
            {
                Ok(_) => CommitOutcome::Committed { seq: global_seq },
                Err(err) => {
                    tracing::error!(
                        doc_id = %doc_id,
                        client_op_id,
                        "Workspace projection writeback failed after ledger commit: {:?}",
                        err
                    );
                    CommitOutcome::WritebackFailed {
                        seq: global_seq,
                        detail: format!("Projection writeback failed after ledger commit: {err}"),
                    }
                }
            };
            emit_commit_outcome(
                CommittedWrite {
                    ch,
                    scope,
                    scope_nonce,
                    doc_id,
                    op,
                    client_id,
                    client_op_id,
                },
                outcome,
            );
        }
        Err(err) => {
            if confirm_existing_client_op(ExistingClientOpCheck {
                state,
                scope,
                ch,
                scope_nonce,
                doc_id,
                op: &op,
                client_id,
                client_op_id,
            }) {
                return;
            }
            tracing::error!("Failed to persist op: {:?}", err);
            reject_edit(
                ch,
                scope_nonce,
                doc_id,
                client_op_id,
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, err.to_string()),
            );
        }
    }
}
