use super::DualChannel;
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::ServerMessage;
use tokio::sync::{broadcast, mpsc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_is_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::SyncHello {
        peer_id: PeerId::new("peer-a"),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 7,
        pub_key: vec![1, 2],
        signature: vec![3, 4],
        vector: VersionVector::default(),
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    tokio::task::yield_now().await;
    match unicast_rx.recv().await {
        Some(ServerMessage::SyncHello {
            peer_id,
            repo_id,
            scope_nonce,
            pub_key,
            signature,
            vector,
        }) => {
            assert_eq!(peer_id.as_str(), "peer-a");
            assert_eq!(repo_id, uuid::Uuid::nil());
            assert_eq!(scope_nonce, 7);
            assert_eq!(pub_key, vec![1, 2]);
            assert_eq!(signature, vec![3, 4]);
            assert_eq!(vector, VersionVector::default());
        }
        other => panic!("expected queued SyncHello, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_followups_are_not_dropped_when_unicast_queue_is_full() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let peer = PeerId::new("peer-a");

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::SyncRequest {
        repo_id: uuid::Uuid::nil(),
        branch: Some(peer.clone()),
        known_vector: VersionVector::default(),
        requests: vec![(peer.clone(), (1, 2))],
    });
    ch.unicast(ServerMessage::SyncSnapshotRequest {
        source_peer_id: peer.clone(),
        repo_id: uuid::Uuid::nil(),
        known_vector: VersionVector::default(),
        reason: None,
    });
    ch.unicast(ServerMessage::SyncPush {
        source_peer_id: peer.clone(),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 7,
        branch: Some(peer.clone()),
        encrypted_payload: vec![],
    });
    ch.unicast(ServerMessage::SyncPushSnapshot {
        source_peer_id: peer.clone(),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 7,
        branch: Some(peer),
        server_vector: VersionVector::default(),
        snapshot_kind: None,
        encrypted_payload: vec![],
    });

    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    for _ in 0..4 {
        tokio::task::yield_now().await;
        let msg = unicast_rx.recv().await.expect("queued sync follow-up");
        assert!(
            matches!(
                msg,
                ServerMessage::SyncRequest { .. }
                    | ServerMessage::SyncSnapshotRequest { .. }
                    | ServerMessage::SyncPush { .. }
                    | ServerMessage::SyncPushSnapshot { .. }
            ),
            "expected queued sync follow-up, got {:?}",
            msg
        );
    }
}
