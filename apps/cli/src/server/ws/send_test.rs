//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! WebSocket outbound delivery regression coverage.

use super::{
    BroadcastFilter, new_unicast_channel, spawn_broadcast_forwarder,
    spawn_unicast_sender_task_with_encoder,
};
use crate::server::session::WsSession;
use axum::extract::ws::Message;
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ConfirmedOp, ServerErrorCode, ServerMessage};
use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::time::{Duration, timeout};

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

    unicast_tx
        .try_send(ServerMessage::Pong)
        .expect("fill unicast queue");
    broadcast_tx
        .send(ServerMessage::CommitAck {
            repo_id: Some(repo_id),
            branch: None,
            scope_nonce: None,
            commit_id: "c1".into(),
            timestamp: 1,
        })
        .expect("broadcast commit ack");

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::CommitAck {
            repo_id: seen_repo_id,
            scope_nonce,
            commit_id,
            ..
        }) => {
            assert_eq!(seen_repo_id, Some(repo_id));
            assert_eq!(scope_nonce, Some(11));
            assert_eq!(commit_id, "c1");
        }
        other => panic!("expected queued CommitAck, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn new_op_broadcasts_are_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, broadcast_rx) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();
    let mut session = WsSession::new();
    let repo_id = uuid::Uuid::nil();
    session.switch_repo("notes".into(), Some(repo_id));
    session.set_scope_nonce(Some(9));
    let filter = BroadcastFilter::for_session(&session);

    spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone(), filter);

    unicast_tx
        .try_send(ServerMessage::Pong)
        .expect("fill unicast queue");
    broadcast_tx
        .send(ServerMessage::NewOp {
            repo_id,
            branch: None,
            scope_nonce: None,
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

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
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

    unicast_tx
        .try_send(ServerMessage::Pong)
        .expect("fill unicast queue");
    broadcast_tx
        .send(ServerMessage::PeerDeleted {
            peer_id: "peer-a".into(),
            scope_nonce: None,
        })
        .expect("broadcast peer deleted");

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
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
async fn non_critical_broadcasts_still_drop_when_unicast_queue_is_full() {
    let (broadcast_tx, broadcast_rx) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = new_unicast_channel();

    spawn_broadcast_forwarder(
        broadcast_rx,
        unicast_tx.clone(),
        BroadcastFilter::allow_all(),
    );

    unicast_tx
        .try_send(ServerMessage::Pong)
        .expect("fill unicast queue");
    broadcast_tx
        .send(ServerMessage::Pong)
        .expect("broadcast pong");

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    assert!(
        unicast_rx.try_recv().is_err(),
        "non-critical broadcast must still be droppable"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lagged_broadcasts_surface_protocol_error() {
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
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(scope_nonce, Some(17));
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("WS broadcast lagged"))
            );
        }
        other => panic!("expected lagged ProtocolError, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serialization_failures_close_unicast_sender_task() {
    let (sink, mut stream) = futures::channel::mpsc::channel::<Message>(1);
    let (unicast_tx, unicast_rx) = new_unicast_channel();

    spawn_unicast_sender_task_with_encoder(sink, unicast_rx, |_| {
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
