//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#tree-projection-contract
//!
use crate::runtime::domain::PendingOpsPreview;
use deve_core::protocol::ServerMessage;

use super::super::super::state::CoreSignals;
use super::super::message_dispatch_sync::{
    handle_merge_complete_message, handle_pending_discarded_message,
    handle_pending_ops_info_message, handle_sync_hello_message, handle_sync_mode_status_message,
};
use super::super::message_runtime_sync::accepts_pending_ops_info;

pub fn route_projection_sync_message(
    msg: ServerMessage,
    signals: CoreSignals,
) -> Option<ServerMessage> {
    match msg {
        ServerMessage::SyncHello {
            peer_id,
            repo_id,
            scope_nonce,
            vector,
            ..
        } => {
            handle_sync_hello_message(peer_id, repo_id, scope_nonce.get(), vector, signals);
            None
        }
        ServerMessage::SyncModeStatus {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            mode,
        } => {
            handle_sync_mode_status_message(
                request_id,
                repo_id,
                branch,
                scope_nonce,
                mode,
                signals,
            );
            None
        }
        ServerMessage::PendingOpsInfo {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            count,
            previews,
        } => {
            if !accepts_pending_ops_info(
                request_id.as_deref(),
                &repo_id,
                &branch,
                scope_nonce,
                signals,
            ) {
                return None;
            }
            let previews = previews.into_iter().map(PendingOpsPreview::from).collect();
            handle_pending_ops_info_message(
                request_id,
                repo_id,
                branch,
                scope_nonce,
                count,
                previews,
                signals,
            );
            None
        }
        ServerMessage::MergeComplete {
            repo_id,
            branch,
            scope_nonce,
            merged_count,
        } => {
            handle_merge_complete_message(repo_id, branch, scope_nonce, merged_count, signals);
            None
        }
        ServerMessage::PendingDiscarded {
            repo_id,
            branch,
            scope_nonce,
        } => {
            handle_pending_discarded_message(repo_id, branch, scope_nonce, signals);
            None
        }
        other => Some(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::state::{CoreSignals, init_signals};
    use deve_core::models::RepoId;
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;

    fn init_runtime() -> (Owner, CoreSignals) {
        let runtime = Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        (runtime, init_signals(connection_status))
    }

    #[test]
    fn route_pending_ops_info_rejects_stale_scope_before_state_update() {
        let (_runtime, signals) = init_runtime();
        let repo_id = RepoId::new_v4();
        signals.set_current_repo_id.set(Some(repo_id.to_string()));
        signals.set_current_scope_nonce.set(7);
        signals
            .set_pending_ops_request_id
            .set(Some("pending-1".into()));

        let routed = route_projection_sync_message(
            ServerMessage::PendingOpsInfo {
                request_id: Some("pending-1".into()),
                repo_id: Some(repo_id),
                branch: None,
                scope_nonce: Some(6),
                count: 2,
                previews: vec![("a.md".into(), "old".into(), "new".into())],
            },
            signals,
        );

        assert!(routed.is_none());
        assert_eq!(signals.pending_ops_count.get_untracked(), 0);
        assert!(signals.pending_ops_previews.get_untracked().is_empty());
        assert_eq!(
            signals.pending_ops_request_id.get_untracked().as_deref(),
            Some("pending-1")
        );
    }
}
