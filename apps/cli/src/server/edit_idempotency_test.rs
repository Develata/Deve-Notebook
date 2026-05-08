//! plan_ref:
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
//! Edit idempotency acceptance for `(client_id, client_op_id)`.

use super::{
    edit_message_test_support::{recv_edit_rejected, send_insert},
    edit_state_test_support::{edit_harness, seed_doc, unicast_channel, writer_browser_session},
};
use deve_core::models::DocId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_client_op_returns_original_ack_without_append() -> anyhow::Result<()> {
    let h = edit_harness(false)?;
    let doc_id = seed_doc(&h.state, "default", "notes/idempotent.md")?;
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session("default", h.default_repo_id, 43);

    send_insert(&h.state, &ch, &mut session, doc_id, 0).await;
    let first_ack = recv_ack_with_seq(&mut uni_rx).await;
    send_insert(&h.state, &ch, &mut session, doc_id, 0).await;
    let duplicate_ack = recv_ack_with_seq(&mut uni_rx).await;

    assert_eq!(first_ack, duplicate_ack);
    let found = h
        .state
        .repo
        .find_client_op_in_local_repo("default", doc_id, 7, 9)?
        .expect("client op index entry");
    assert_eq!(found.1.seq, first_ack.seq);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_client_op_with_different_op_is_rejected() -> anyhow::Result<()> {
    let h = edit_harness(false)?;
    let doc_id = seed_doc(&h.state, "default", "notes/conflict.md")?;
    let (ch, mut uni_rx) = unicast_channel(&h.state);
    let mut session = writer_browser_session("default", h.default_repo_id, 47);

    send_insert(&h.state, &ch, &mut session, doc_id, 0).await;
    let first_ack = recv_ack_with_seq(&mut uni_rx).await;
    send_insert(&h.state, &ch, &mut session, doc_id, 1).await;
    let (scope_nonce, rejected_doc_id, client_op_id, error) =
        recv_edit_rejected(&mut uni_rx).await;

    assert_eq!(scope_nonce, 47);
    assert_eq!(rejected_doc_id, doc_id);
    assert_eq!(client_op_id, 9);
    assert_eq!(error.code, ServerErrorCode::SyncEditRejected);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("client_op_id conflicts")),
        "unexpected detail: {:?}",
        error.detail
    );
    let found = h
        .state
        .repo
        .find_client_op_in_local_repo("default", doc_id, 7, 9)?
        .expect("client op index entry");
    assert_eq!(found.1.seq, first_ack.seq);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct AckSummary {
    scope_nonce: Option<u64>,
    doc_id: DocId,
    seq: u64,
    client_op_id: u64,
}

async fn recv_ack_with_seq(rx: &mut mpsc::Receiver<ServerMessage>) -> AckSummary {
    match rx.recv().await {
        Some(ServerMessage::Ack {
            scope_nonce,
            doc_id,
            seq,
            client_op_id,
            ..
        }) => AckSummary {
            scope_nonce,
            doc_id,
            seq,
            client_op_id,
        },
        other => panic!("expected Ack, got {:?}", other),
    }
}
