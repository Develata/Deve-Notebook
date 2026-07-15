use super::DualChannel;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::{MergeConflictAction, ServerMessage};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

#[tokio::test]
async fn response_control_messages_retire_when_regular_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, _unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let mut retired = ch.retirement_receiver();
    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::Ack {
        repo_id: uuid::Uuid::nil(),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: Some(5),
        doc_id: DocId::new(),
        seq: 42,
        client_op_id: 9,
    });
    retired.changed().await.expect("retirement");
    assert!(*retired.borrow());
}

#[tokio::test]
async fn merge_conflict_uses_dedicated_bounded_diff_queue() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let (diff_tx, mut diff_rx) = mpsc::channel(1);
    let ch = DualChannel::with_diff_channel(broadcast_tx, unicast_tx, diff_tx);
    let doc_id = DocId::new();
    let projection = Arc::new(
        deve_core::source_control::diff_projection::compute_diff_projection(
            "local".into(),
            "remote".into(),
        )
        .unwrap(),
    );

    ch.unicast(ServerMessage::Pong);
    assert!(
        ch.diff_unicast(ServerMessage::MergeConflict {
            repo_id: Some(uuid::Uuid::nil()),
            branch: Some(PeerId::new("peer-a")),
            scope_nonce: Some(5),
            doc_id,
            path: "notes/a.md".into(),
            projection,
            result_content: "base".into(),
            actions: vec![
                MergeConflictAction::AcceptCurrent,
                MergeConflictAction::AcceptIncoming,
                MergeConflictAction::AcceptBoth,
            ],
            conflicts: Vec::new(),
        })
        .await
    );

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    assert!(matches!(
        diff_rx.recv().await,
        Some(ServerMessage::MergeConflict { doc_id: seen, .. }) if seen == doc_id
    ));
}
