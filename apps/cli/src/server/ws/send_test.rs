use super::{BroadcastFilter, new_unicast_channel, spawn_broadcast_forwarder};
use crate::server::session::WsSession;
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ConfirmedOp, ServerMessage};
use tokio::sync::broadcast;

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
