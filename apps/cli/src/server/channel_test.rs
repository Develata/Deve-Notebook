use super::DualChannel;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protocol_errors_are_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.send_protocol_error(ServerError::new(ServerErrorCode::RequestFailed));

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
        }
        other => panic!("expected queued ProtocolError, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_critical_unicast_messages_still_drop_when_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::Pong);

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    assert!(
        unicast_rx.try_recv().is_err(),
        "second pong should be dropped"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scope_switch_messages_are_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::BranchSwitched {
        peer_id: Some("peer-a".into()),
        success: true,
        switch_nonce: Some(7),
    });
    ch.unicast(ServerMessage::RepoSwitched {
        branch: Some("peer-a".into()),
        name: "wiki".into(),
        uuid: "repo-1".into(),
        switch_nonce: Some(7),
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    let first = unicast_rx.recv().await.expect("first queued control message");
    tokio::task::yield_now().await;
    let second = unicast_rx.recv().await.expect("second queued control message");

    let mut saw_branch = false;
    let mut saw_repo = false;
    for msg in [first, second] {
        match msg {
            ServerMessage::BranchSwitched {
                peer_id,
                success,
                switch_nonce,
            } => {
                assert_eq!(peer_id.as_deref(), Some("peer-a"));
                assert!(success);
                assert_eq!(switch_nonce, Some(7));
                saw_branch = true;
            }
            ServerMessage::RepoSwitched {
                branch,
                name,
                uuid,
                switch_nonce,
            } => {
                assert_eq!(branch.as_deref(), Some("peer-a"));
                assert_eq!(name, "wiki");
                assert_eq!(uuid, "repo-1");
                assert_eq!(switch_nonce, Some(7));
                saw_repo = true;
            }
            other => panic!("expected queued scope control message, got {:?}", other),
        }
    }
    assert!(saw_branch, "BranchSwitched must not be dropped");
    assert!(saw_repo, "RepoSwitched must not be dropped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn write_ready_is_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::WriteReady {
        peer_id: PeerId::new("peer-a"),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 9,
        branch: Some(PeerId::new("peer-a")),
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::WriteReady {
            peer_id,
            repo_id,
            scope_nonce,
            branch,
        }) => {
            assert_eq!(peer_id.as_str(), "peer-a");
            assert_eq!(repo_id, uuid::Uuid::nil());
            assert_eq!(scope_nonce, 9);
            assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
        }
        other => panic!("expected queued WriteReady, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_messages_are_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::KeyProvide {
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 3,
        branch: Some(PeerId::new("peer-a")),
        repo_key: vec![1, 2, 3],
    });
    ch.unicast(ServerMessage::KeyDenied {
        repo_id: Some(uuid::Uuid::nil()),
        scope_nonce: 3,
        branch: Some(PeerId::new("peer-a")),
        error: ServerError::new(ServerErrorCode::SyncPeerUnauthenticated),
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    let first = unicast_rx.recv().await.expect("first queued key message");
    tokio::task::yield_now().await;
    let second = unicast_rx.recv().await.expect("second queued key message");

    let mut saw_provide = false;
    let mut saw_denied = false;
    for msg in [first, second] {
        match msg {
            ServerMessage::KeyProvide {
                repo_id,
                scope_nonce,
                branch,
                repo_key,
            } => {
                assert_eq!(repo_id, uuid::Uuid::nil());
                assert_eq!(scope_nonce, 3);
                assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
                assert_eq!(repo_key, vec![1, 2, 3]);
                saw_provide = true;
            }
            ServerMessage::KeyDenied {
                repo_id,
                scope_nonce,
                branch,
                error,
            } => {
                assert_eq!(repo_id, Some(uuid::Uuid::nil()));
                assert_eq!(scope_nonce, 3);
                assert_eq!(branch.as_ref().map(PeerId::as_str), Some("peer-a"));
                assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
                saw_denied = true;
            }
            other => panic!("expected queued key message, got {:?}", other),
        }
    }
    assert!(saw_provide, "KeyProvide must not be dropped");
    assert!(saw_denied, "KeyDenied must not be dropped");
}
