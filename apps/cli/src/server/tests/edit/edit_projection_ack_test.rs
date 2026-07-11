use super::{
    edit_message_test_support::send_insert,
    edit_state_test_support::{edit_harness, seed_doc, unicast_channel, writer_browser_session},
};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc::error::TryRecvError;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn edit_acknowledges_ledger_commit_when_workspace_writeback_fails() -> anyhow::Result<()> {
    let h = edit_harness(false)?;
    let doc_id = seed_doc(&h.state, "default", "notes/a.md")?;
    std::fs::create_dir_all(h.state.repo.local_repo_workspace_path("default", "notes/a.md")?)?;
    let mut broadcast_rx = h.state.tx.subscribe();
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session("default", h.default_repo_id, 37);

    send_insert(&h.state, &ch, &mut session, doc_id, 0).await;

    let (global_seq, entry) = h
        .state
        .repo
        .find_client_op_in_local_repo("default", 7, 9)?
        .expect("committed client op");
    assert_eq!(entry.origin_peer_id, *h.state.repo.local_peer_id());
    assert!(entry.peer_seq.get() > 0);
    match uni_rx.recv().await {
        Some(ServerMessage::Ack {
            scope_nonce,
            doc_id: ack_doc_id,
            seq,
            client_op_id,
            ..
        }) => {
            assert_eq!(scope_nonce, Some(37));
            assert_eq!(ack_doc_id, doc_id);
            assert_eq!(seq, global_seq);
            assert_eq!(client_op_id, 9);
        }
        other => panic!("expected Ack, got {other:?}"),
    }
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error,
            scope_nonce,
            switch_nonce,
        }) => {
            assert_eq!(scope_nonce, Some(37));
            assert_eq!(switch_nonce, None);
            assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("Projection writeback failed"))
            );
        }
        other => panic!("expected projection writeback ProtocolError, got {other:?}"),
    }
    match broadcast_rx.recv().await? {
        ServerMessage::NewOp {
            doc_id: broadcast_doc_id,
            entry,
            ..
        } => {
            assert_eq!(broadcast_doc_id, doc_id);
            assert_eq!(entry.seq, global_seq);
        }
        other => panic!("expected NewOp broadcast, got {:?}", other),
    }
    assert!(matches!(uni_rx.try_recv(), Err(TryRecvError::Empty)));
    assert!(
        h.state
            .repo
            .find_client_op_in_local_repo("default", 7, 9)?
            .is_some()
    );
    assert!(h.state.sync_manager.is_projection_degraded("default"));
    Ok(())
}
