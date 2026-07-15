//! plan_ref:
//!   - 07_network#server-ws-runtime

use deve_core::protocol::ServerMessage;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use crate::server::metrics;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeliveryOutcome {
    Queued,
    Dropped,
    Closed,
    CriticalQueueFull,
}

pub(crate) fn try_send_with_delivery_class(
    tx: &mpsc::Sender<ServerMessage>,
    msg: ServerMessage,
    must_deliver: bool,
) -> DeliveryOutcome {
    match tx.try_send(msg) {
        Ok(()) => DeliveryOutcome::Queued,
        Err(error) => match error {
            TrySendError::Full(_) if must_deliver => {
                metrics::record_critical_session_retirement();
                let counters = metrics::delivery_metrics_snapshot();
                tracing::warn!(
                    critical_session_retirements = counters.critical_session_retirements,
                    "Critical unicast queue full; retiring session for resync"
                );
                DeliveryOutcome::CriticalQueueFull
            }
            TrySendError::Full(_) => {
                metrics::record_noncritical_unicast_drop();
                let counters = metrics::delivery_metrics_snapshot();
                tracing::warn!(
                    noncritical_unicast_drops = counters.noncritical_unicast_drops,
                    "Unicast channel full; dropping non-critical message"
                );
                DeliveryOutcome::Dropped
            }
            TrySendError::Closed(_) => {
                tracing::debug!("Unicast channel closed; dropping message");
                DeliveryOutcome::Closed
            }
        },
    }
}

pub(super) fn send_unicast(
    tx: &mpsc::Sender<ServerMessage>,
    msg: ServerMessage,
) -> DeliveryOutcome {
    let must_deliver = must_deliver_unicast(&msg);
    try_send_with_delivery_class(tx, msg, must_deliver)
}

fn must_deliver_unicast(msg: &ServerMessage) -> bool {
    matches!(
        msg,
        ServerMessage::ProtocolError { .. }
            | ServerMessage::EditRejected { .. }
            | ServerMessage::SyncHello { .. }
            | ServerMessage::SyncRequest { .. }
            | ServerMessage::SyncSnapshotRequest { .. }
            | ServerMessage::SyncPush { .. }
            | ServerMessage::SyncPushSnapshot { .. }
            | ServerMessage::BranchSwitched { .. }
            | ServerMessage::RepoSwitched { .. }
            | ServerMessage::WriteReady { .. }
            | ServerMessage::KeyProvide { .. }
            | ServerMessage::KeyDenied { .. }
            | ServerMessage::Ack { .. }
            | ServerMessage::Snapshot { .. }
            | ServerMessage::History { .. }
            | ServerMessage::RepoList { .. }
            | ServerMessage::DocList { .. }
            | ServerMessage::TreeUpdate { .. }
            | ServerMessage::ShadowList { .. }
            | ServerMessage::PluginResponse { .. }
            | ServerMessage::SearchResults { .. }
            | ServerMessage::SyncModeStatus { .. }
            | ServerMessage::PendingOpsInfo { .. }
            | ServerMessage::PendingDiscarded { .. }
            | ServerMessage::ChangesList { .. }
            | ServerMessage::ExternalApplyAck { .. }
            | ServerMessage::ProjectionRecoveryRequired(_)
            | ServerMessage::StageAck { .. }
            | ServerMessage::UnstageAck { .. }
            | ServerMessage::DiscardAck { .. }
            | ServerMessage::CommitHistory { .. }
            | ServerMessage::ConflictResolved { .. }
    )
}
