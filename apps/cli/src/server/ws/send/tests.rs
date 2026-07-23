//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! WebSocket outbound delivery regression coverage.

use super::{
    BroadcastFilter, UNICAST_CAPACITY, encode_server_message, new_diff_unicast_channel,
    new_unicast_channel, spawn_broadcast_forwarder, spawn_unicast_sender_task_with_encoder,
};
use crate::server::metrics;
use crate::server::session::WsSession;
use axum::extract::ws::Message;
use deve_core::models::{DocId, Op};
use deve_core::protocol::frame::decode_server_binary;
use deve_core::protocol::{
    ConfirmedOp, ProjectionRecoveryCause, RepoListEntry, RepoReadiness, ServerMessage,
};
use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::time::{Duration, timeout};

#[test]
fn outbound_encoder_writes_versioned_server_frame() {
    let bytes = encode_server_message(&ServerMessage::Pong).expect("encode frame");
    assert!(matches!(
        decode_server_binary(&bytes),
        Ok(ServerMessage::Pong)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn critical_repo_scoped_broadcasts_are_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, broadcast_rx) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();
    let mut session = WsSession::new();
    let repo_id = uuid::Uuid::nil();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(11));
    let filter = BroadcastFilter::for_session(&session);

    spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone(), filter);

    for _ in 0..UNICAST_CAPACITY {
        unicast_tx
            .try_send(ServerMessage::Pong)
            .expect("fill unicast queue");
    }
    broadcast_tx
        .send(ServerMessage::CommitAck {
            repo_id: Some(repo_id),
            branch: None,
            scope_nonce: None,
            commit_id: "c1".into(),
            timestamp: 1,
        })
        .expect("broadcast commit ack");
    broadcast_tx
        .send(ServerMessage::CommitAck {
            repo_id: Some(repo_id),
            branch: None,
            scope_nonce: None,
            commit_id: "c2".into(),
            timestamp: 2,
        })
        .expect("broadcast second commit ack");

    for _ in 0..UNICAST_CAPACITY {
        assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    }
    tokio::task::yield_now().await;
    for expected in ["c1", "c2"] {
        match unicast_rx.recv().await {
            Some(ServerMessage::CommitAck {
                repo_id: seen_repo_id,
                scope_nonce,
                commit_id,
                ..
            }) => {
                assert_eq!(seen_repo_id, Some(repo_id));
                assert_eq!(scope_nonce, Some(11));
                assert_eq!(commit_id, expected);
            }
            other => panic!("expected ordered CommitAck, got {:?}", other),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn new_op_broadcast_uses_recipient_nonce_when_unicast_queue_is_full() {
    let (broadcast_tx, broadcast_rx) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();
    let mut session = WsSession::new();
    let repo_id = uuid::Uuid::nil();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);

    spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone(), filter);

    for _ in 0..UNICAST_CAPACITY {
        unicast_tx
            .try_send(ServerMessage::Pong)
            .expect("fill unicast queue");
    }
    broadcast_tx
        .send(ServerMessage::NewOp {
            repo_id,
            branch: None,
            scope_nonce: Some(7),
            doc_id: DocId::new(),
            entry: ConfirmedOp::new(
                1,
                Op::Insert {
                    pos: 0,
                    content: "x".into(),
                },
                None,
            ),
        })
        .expect("broadcast new op");

    for _ in 0..UNICAST_CAPACITY {
        assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    }
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::NewOp {
            repo_id: seen_repo_id,
            scope_nonce,
            ..
        }) => {
            assert_eq!(seen_repo_id, repo_id);
            assert_eq!(scope_nonce, Some(9));
        }
        other => panic!("expected queued NewOp, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peer_deleted_broadcasts_are_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, broadcast_rx) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    session.set_scope_nonce(Some(13));
    let filter = BroadcastFilter::for_session(&session);

    spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone(), filter);

    for _ in 0..UNICAST_CAPACITY {
        unicast_tx
            .try_send(ServerMessage::Pong)
            .expect("fill unicast queue");
    }
    broadcast_tx
        .send(ServerMessage::PeerDeleted {
            peer_id: "peer-a".into(),
            scope_nonce: None,
        })
        .expect("broadcast peer deleted");

    for _ in 0..UNICAST_CAPACITY {
        assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    }
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::PeerDeleted {
            peer_id,
            scope_nonce,
        }) => {
            assert_eq!(peer_id, "peer-a");
            assert_eq!(scope_nonce, Some(13));
        }
        other => panic!("expected queued PeerDeleted, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repo_list_broadcasts_are_not_dropped_for_no_scope_browser() {
    let (broadcast_tx, broadcast_rx) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(13));
    let filter = BroadcastFilter::for_session(&session);
    let repo_id = uuid::Uuid::new_v4();

    spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone(), filter);

    for _ in 0..UNICAST_CAPACITY {
        unicast_tx
            .try_send(ServerMessage::Pong)
            .expect("fill unicast queue");
    }
    broadcast_tx
        .send(ServerMessage::RepoList {
            request_id: None,
            branch: None,
            scope_nonce: None,
            repo_entries: vec![RepoListEntry {
                repo_id,
                display_alias: "created".into(),
                alias_revision: 1,
                readiness: RepoReadiness::Mounted,
            }],
        })
        .expect("broadcast repo list");

    for _ in 0..UNICAST_CAPACITY {
        assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    }
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::RepoList {
            request_id,
            branch,
            scope_nonce,
            repo_entries,
        }) => {
            assert_eq!(request_id, None);
            assert_eq!(branch, None);
            assert_eq!(scope_nonce, Some(13));
            assert_eq!(repo_entries.len(), 1);
            assert_eq!(repo_entries[0].repo_id, repo_id);
        }
        other => panic!("expected queued RepoList, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_critical_broadcasts_still_drop_when_unicast_queue_is_full() {
    let before = metrics::delivery_metrics_snapshot();
    let (broadcast_tx, broadcast_rx) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();

    spawn_broadcast_forwarder(
        broadcast_rx,
        unicast_tx.clone(),
        BroadcastFilter::allow_all(),
    );

    for _ in 0..UNICAST_CAPACITY {
        unicast_tx
            .try_send(ServerMessage::Pong)
            .expect("fill unicast queue");
    }
    broadcast_tx
        .send(ServerMessage::Pong)
        .expect("broadcast pong");

    timeout(Duration::from_secs(1), async {
        while metrics::delivery_metrics_snapshot().noncritical_broadcast_drops
            <= before.noncritical_broadcast_drops
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("broadcast drop metric");
    for _ in 0..UNICAST_CAPACITY {
        assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    }
    assert!(
        unicast_rx.try_recv().is_err(),
        "non-critical broadcast must still be droppable"
    );
    assert!(
        metrics::delivery_metrics_snapshot().noncritical_broadcast_drops
            > before.noncritical_broadcast_drops
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lagged_broadcasts_surface_scoped_recovery() {
    let before = metrics::delivery_metrics_snapshot();
    let (broadcast_tx, broadcast_rx) = broadcast::channel(1);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();
    let mut session = WsSession::new();
    session.switch_repo("notes".into(), Some(uuid::Uuid::nil()));
    session.set_scope_nonce(Some(17));

    broadcast_tx
        .send(ServerMessage::Pong)
        .expect("seed first broadcast");
    broadcast_tx
        .send(ServerMessage::Pong)
        .expect("seed second broadcast");

    spawn_broadcast_forwarder(
        broadcast_rx,
        unicast_tx,
        BroadcastFilter::for_session(&session),
    );

    match unicast_rx.recv().await {
        Some(ServerMessage::ProjectionRecoveryRequired(recovery)) => {
            assert_eq!(recovery.repo_id, uuid::Uuid::nil());
            assert_eq!(recovery.scope_nonce, Some(17));
            assert!(matches!(
                recovery.cause,
                ProjectionRecoveryCause::BroadcastGap { skipped: 1 }
            ));
        }
        other => panic!("expected lagged projection recovery, got {:?}", other),
    }
    let after = metrics::delivery_metrics_snapshot();
    assert!(after.broadcast_lag_events > before.broadcast_lag_events);
    assert!(after.broadcast_recoveries > before.broadcast_recoveries);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serialization_failures_close_unicast_sender_task() {
    let (sink, mut stream) = futures::channel::mpsc::channel::<Message>(1);
    let (unicast_tx, unicast_rx) = new_unicast_channel();
    let (_diff_tx, diff_rx) = new_diff_unicast_channel();

    spawn_unicast_sender_task_with_encoder(sink, unicast_rx, diff_rx, |_| {
        Err("synthetic serialization failure".into())
    });

    unicast_tx
        .send(ServerMessage::Pong)
        .await
        .expect("queue first message");
    unicast_tx
        .send(ServerMessage::Pong)
        .await
        .expect("queue second message");
    drop(unicast_tx);

    let next = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("sender task must stop after serialization failure");
    assert!(
        next.is_none(),
        "sender task must close sink after encode failure"
    );
}

#[test]
fn diff_unicast_channel_has_exactly_one_waiting_slot() {
    let (tx, mut rx) = new_diff_unicast_channel();
    tx.try_send(ServerMessage::Pong).expect("first diff slot");
    assert!(matches!(
        tx.try_send(ServerMessage::Pong),
        Err(tokio::sync::mpsc::error::TrySendError::Full(_))
    ));
    assert!(matches!(rx.try_recv(), Ok(ServerMessage::Pong)));
}
