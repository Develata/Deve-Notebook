//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 10_rendering#document-authority-bridge
//!
//! Single emission point for the writer-facing result of an authority edit.

use crate::server::{channel::DualChannel, repo_scope::ResolvedRepo};
use deve_core::models::DocId;
use deve_core::protocol::{ServerError, ServerMessage};

use super::edit_support::reject_edit;
use crate::server::repo_mutation::{MutationExecution, RepoMutationGateError};

pub(super) struct WriteConfirmation<'a> {
    pub(super) ch: &'a DualChannel,
    pub(super) scope: &'a ResolvedRepo,
    pub(super) scope_nonce: u64,
    pub(super) doc_id: DocId,
    pub(super) client_op_id: u64,
}

pub(super) fn emit_write_confirmation(
    confirmation: WriteConfirmation<'_>,
    execution: Result<MutationExecution<u64, Option<ServerError>>, RepoMutationGateError>,
) {
    let WriteConfirmation {
        ch,
        scope,
        scope_nonce,
        doc_id,
        client_op_id,
    } = confirmation;
    let ack = |seq| {
        ch.unicast(ServerMessage::Ack {
            repo_id: scope.repo_id,
            branch: scope.branch.clone(),
            scope_nonce: Some(scope_nonce),
            doc_id,
            seq,
            client_op_id,
        });
    };

    match execution {
        Ok(MutationExecution::Committed { value: seq, .. }) => ack(seq),
        Ok(MutationExecution::ProjectionDegraded {
            value: seq, error, ..
        }) => {
            ack(seq);
            if let Some(error) = error {
                ch.send_protocol_error_with_scope_nonce(error, Some(scope_nonce));
            }
        }
        Ok(MutationExecution::NotCommitted(Some(error)))
        | Ok(MutationExecution::CommittedPartial {
            error: Some(error), ..
        }) => reject_edit(ch, scope_nonce, doc_id, client_op_id, error),
        Ok(MutationExecution::NotCommitted(None))
        | Ok(MutationExecution::CommittedPartial { error: None, .. }) => {}
        Err(error) => reject_edit(ch, scope_nonce, doc_id, client_op_id, error.server_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::repo_scope::ResolvedRepo;
    use deve_core::models::RepoId;
    use deve_core::protocol::ServerErrorCode;
    use tokio::sync::{broadcast, mpsc};

    #[tokio::test]
    async fn workspace_ingestion_error_mapping_editor_uses_edit_rejected() {
        let (tx, _) = broadcast::channel(4);
        let (unicast_tx, mut unicast_rx) = mpsc::channel(4);
        let ch = DualChannel::new(tx, unicast_tx);
        let scope = ResolvedRepo {
            repo_id: RepoId::new_v4(),
            repo_name: "notes".into(),
            branch: None,
        };
        let doc_id = DocId::new();

        emit_write_confirmation(
            WriteConfirmation {
                ch: &ch,
                scope: &scope,
                scope_nonce: 9,
                doc_id,
                client_op_id: 11,
            },
            Err(RepoMutationGateError::WorkspaceIngestionUnavailable),
        );

        match unicast_rx.recv().await.expect("edit rejection") {
            ServerMessage::EditRejected { error, .. } => assert_eq!(
                error.code,
                ServerErrorCode::StorageWorkspaceIngestionUnavailable
            ),
            other => panic!("expected EditRejected, got {other:?}"),
        }
    }
}
