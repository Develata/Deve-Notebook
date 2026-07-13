//! plan_ref:
//!   - 07_network#server-ws-runtime

use deve_core::protocol::ServerMessage;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

pub(crate) fn try_send_with_delivery_class(
    tx: &mpsc::Sender<ServerMessage>,
    msg: ServerMessage,
    must_deliver: bool,
) {
    if let Err(error) = tx.try_send(msg) {
        match error {
            TrySendError::Full(msg) if must_deliver => schedule_must_deliver(tx.clone(), msg),
            TrySendError::Full(_) => {
                tracing::warn!("Unicast channel full; dropping message");
            }
            TrySendError::Closed(_) => {
                tracing::debug!("Unicast channel closed; dropping message");
            }
        }
    }
}

pub(super) fn send_unicast(tx: &mpsc::Sender<ServerMessage>, msg: ServerMessage) {
    let must_deliver = must_deliver_unicast(&msg);
    try_send_with_delivery_class(tx, msg, must_deliver);
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
            | ServerMessage::StageAck { .. }
            | ServerMessage::UnstageAck { .. }
            | ServerMessage::DiscardAck { .. }
            | ServerMessage::CommitHistory { .. }
            | ServerMessage::ConflictResolved { .. }
    )
}

fn schedule_must_deliver(tx: mpsc::Sender<ServerMessage>, msg: ServerMessage) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            let _ = tx.send(msg).await;
        });
    } else if tx.blocking_send(msg).is_err() {
        tracing::debug!("Unicast channel closed outside runtime; dropping critical message");
    }
}
