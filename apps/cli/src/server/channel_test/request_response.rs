use super::DualChannel;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::{MergeConflictAction, ServerMessage};
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ack_is_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let doc_id = DocId::new();

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::Ack {
        repo_id: uuid::Uuid::nil(),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: Some(5),
        doc_id,
        seq: 42,
        client_op_id: 9,
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::Ack {
            repo_id,
            branch,
            scope_nonce,
            doc_id: received_doc_id,
            seq,
            client_op_id,
        }) => {
            assert_eq!(repo_id, uuid::Uuid::nil());
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
            assert_eq!(scope_nonce, Some(5));
            assert_eq!(received_doc_id, doc_id);
            assert_eq!(seq, 42);
            assert_eq!(client_op_id, 9);
        }
        other => panic!("expected queued Ack, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_is_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let doc_id = DocId::new();

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::Snapshot {
        repo_id: uuid::Uuid::nil(),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: Some(5),
        doc_id,
        request_id: 11,
        content: "hello".into(),
        base_seq: 3,
        version: 4,
        delta_ops: vec![],
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::Snapshot {
            repo_id,
            branch,
            scope_nonce,
            doc_id: received_doc_id,
            request_id,
            content,
            base_seq,
            version,
            delta_ops,
        }) => {
            assert_eq!(repo_id, uuid::Uuid::nil());
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
            assert_eq!(scope_nonce, Some(5));
            assert_eq!(received_doc_id, doc_id);
            assert_eq!(request_id, 11);
            assert_eq!(content, "hello");
            assert_eq!(base_seq, 3);
            assert_eq!(version, 4);
            assert!(delta_ops.is_empty());
        }
        other => panic!("expected queued Snapshot, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_discarded_is_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::PendingDiscarded {
        repo_id: Some(uuid::Uuid::nil()),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: Some(5),
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::PendingDiscarded {
            repo_id,
            branch,
            scope_nonce,
        }) => {
            assert_eq!(repo_id, Some(uuid::Uuid::nil()));
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
            assert_eq!(scope_nonce, Some(5));
        }
        other => panic!("expected queued PendingDiscarded, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conflict_resolved_is_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::ConflictResolved {
        repo_id: Some(uuid::Uuid::nil()),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: Some(5),
        path: "notes/a.md".into(),
        resolution: "KeepLedger".into(),
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::ConflictResolved {
            repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
        }) => {
            assert_eq!(repo_id, Some(uuid::Uuid::nil()));
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
            assert_eq!(scope_nonce, Some(5));
            assert_eq!(path, "notes/a.md");
            assert_eq!(resolution, "KeepLedger");
        }
        other => panic!("expected queued ConflictResolved, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn merge_conflict_is_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let doc_id = DocId::new();

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::MergeConflict {
        repo_id: Some(uuid::Uuid::nil()),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: Some(5),
        doc_id,
        path: "notes/a.md".into(),
        current_content: "local".into(),
        incoming_content: "remote".into(),
        result_content: "base".into(),
        actions: vec![
            MergeConflictAction::AcceptCurrent,
            MergeConflictAction::AcceptIncoming,
            MergeConflictAction::AcceptBoth,
        ],
        conflicts: Vec::new(),
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::MergeConflict {
            repo_id,
            branch,
            scope_nonce,
            doc_id: received_doc_id,
            path,
            actions,
            ..
        }) => {
            assert_eq!(repo_id, Some(uuid::Uuid::nil()));
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
            assert_eq!(scope_nonce, Some(5));
            assert_eq!(received_doc_id, doc_id);
            assert_eq!(path, "notes/a.md");
            assert_eq!(actions.len(), 3);
        }
        other => panic!("expected queued MergeConflict, got {:?}", other),
    }
}
