use crate::api::WsService;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

pub(super) fn request_repo_sync_state(ws: &WsService, signals: CoreSignals) {
    if !should_request_repo_sync_state(signals.active_branch.get_untracked()) {
        return;
    }
    let scope_nonce = signals.current_scope_nonce.get_untracked();
    let sync_mode_request_id = next_request_id();
    let pending_ops_request_id = next_request_id();
    signals
        .set_sync_mode_request_id
        .set(Some(sync_mode_request_id.clone()));
    signals
        .set_pending_ops_request_id
        .set(Some(pending_ops_request_id.clone()));
    ws.send(ClientMessage::GetSyncMode {
        request_id: sync_mode_request_id,
        scope_nonce: Some(scope_nonce),
    });
    ws.send(ClientMessage::GetPendingOps {
        request_id: pending_ops_request_id,
        scope_nonce: Some(scope_nonce),
    });
}

pub(super) fn request_repo_list(ws: &WsService, signals: CoreSignals) {
    let request_id = next_request_id();
    signals
        .set_repo_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListRepos {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce.get_untracked()),
    });
}

pub(super) fn next_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub(super) fn should_request_repo_sync_state(active_branch: Option<PeerId>) -> bool {
    active_branch.is_none()
}
