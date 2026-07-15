use super::DualChannel;
use crate::server::metrics;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use tokio::sync::{broadcast, mpsc};

mod request_response;
mod sync_control;

#[tokio::test]
async fn critical_unicast_queue_full_retires_session_without_background_send() {
    let before = metrics::delivery_metrics_snapshot();
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let mut retired = ch.retirement_receiver();

    ch.unicast(ServerMessage::Pong);
    ch.send_protocol_error(ServerError::new(ServerErrorCode::RequestFailed));

    retired.changed().await.expect("retirement signal");
    assert!(*retired.borrow());
    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    assert!(unicast_rx.try_recv().is_err());
    assert!(
        metrics::delivery_metrics_snapshot().critical_session_retirements
            > before.critical_session_retirements
    );
}

#[tokio::test]
async fn non_critical_unicast_queue_full_drops_without_retiring_session() {
    let before = metrics::delivery_metrics_snapshot();
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let retired = ch.retirement_receiver();

    ch.unicast(ServerMessage::Pong);
    ch.unicast(ServerMessage::Pong);

    assert!(!*retired.borrow());
    assert!(matches!(unicast_rx.recv().await, Some(ServerMessage::Pong)));
    assert!(unicast_rx.try_recv().is_err());
    assert!(
        metrics::delivery_metrics_snapshot().noncritical_unicast_drops
            > before.noncritical_unicast_drops
    );
}

#[tokio::test]
async fn critical_unicast_is_queued_when_capacity_is_available() {
    let (broadcast_tx, _) = broadcast::channel(4);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(1);
    let ch = DualChannel::new(broadcast_tx, unicast_tx);
    let retired = ch.retirement_receiver();

    ch.send_protocol_error(ServerError::new(ServerErrorCode::RequestFailed));

    assert!(!*retired.borrow());
    assert!(matches!(
        unicast_rx.recv().await,
        Some(ServerMessage::ProtocolError { .. })
    ));
}
