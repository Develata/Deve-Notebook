use super::DualChannel;
use deve_core::models::{PeerFactSeq, PeerId};
use deve_core::protocol::ServerMessage;
use tokio::sync::{broadcast, mpsc};

#[tokio::test]
async fn sync_control_queue_full_retires_instead_of_spawning_deferred_send() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, _unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let mut retired = ch.retirement_receiver();
    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::SyncRequest {
        repo_id: uuid::Uuid::nil(),
        branch: None,
        known_vector: Default::default(),
        requests: vec![(
            PeerId::new("peer-a"),
            (PeerFactSeq::new(1), PeerFactSeq::new(2)),
        )],
    });
    retired.changed().await.expect("retirement");
    assert!(*retired.borrow());
}
