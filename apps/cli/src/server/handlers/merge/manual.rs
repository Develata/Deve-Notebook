use super::errors;
use super::manual_support::{load_engine, sync_mode_label};
use super::scope::{resolve_read_repo_id, resolve_write_repo_id};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::config::SyncMode;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_get_sync_mode(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
) {
    let scope_nonce = Some(session.scope_nonce());
    let Some(repo_id) = resolve_read_repo_id(state, ch, session) else {
        return;
    };
    let Some(engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    ch.unicast(ServerMessage::SyncModeStatus {
        request_id: Some(request_id),
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
        mode: sync_mode_label(engine.sync_mode()),
    });
}

pub(super) async fn handle_set_sync_mode(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    mode: String,
) {
    let scope_nonce = Some(session.scope_nonce());
    let Some(repo_id) = resolve_write_repo_id(state, ch, session) else {
        return;
    };
    let new_mode = match mode.to_lowercase().as_str() {
        "auto" => SyncMode::Auto,
        "manual" => SyncMode::Manual,
        _ => return errors::request_failed(ch, format!("Invalid sync mode: {}", mode)),
    };
    let Some(mut engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    engine.set_sync_mode(new_mode);
    tracing::info!("SetSyncMode: {:?}", new_mode);
    ch.unicast(ServerMessage::SyncModeStatus {
        request_id: None,
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
        mode: sync_mode_label(new_mode),
    });
}

pub(super) async fn handle_get_pending_ops(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
) {
    let scope_nonce = Some(session.scope_nonce());
    let Some(repo_id) = resolve_read_repo_id(state, ch, session) else {
        return;
    };
    let Some(engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    let pending_count = engine.pending_ops_count();
    let previews = if pending_count > 0 {
        vec![(
            "(pending operations)".to_string(),
            "...".to_string(),
            "...".to_string(),
        )]
    } else {
        Vec::new()
    };
    ch.unicast(ServerMessage::PendingOpsInfo {
        request_id: Some(request_id),
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
        count: pending_count as u32,
        previews,
    });
}

pub(super) async fn handle_confirm_merge(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
) {
    let scope_nonce = Some(session.scope_nonce());
    let Some(repo_id) = resolve_write_repo_id(state, ch, session) else {
        return;
    };
    let Some(mut engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    match engine.merge_pending() {
        Ok(count) => {
            tracing::info!("Merged {} pending operations", count);
            ch.broadcast(ServerMessage::MergeComplete {
                repo_id: Some(repo_id),
                branch: session.active_branch.clone(),
                scope_nonce,
                merged_count: count as u32,
            });
        }
        Err(e) => {
            tracing::error!("Merge failed: {:?}", e);
            errors::classified_failure(ch, format!("Merge failed: {}", e));
        }
    }
}

pub(super) async fn handle_discard_pending(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
) {
    let scope_nonce = Some(session.scope_nonce());
    let Some(repo_id) = resolve_write_repo_id(state, ch, session) else {
        return;
    };
    let Some(mut engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    engine.clear_pending();
    tracing::info!("Discarded all pending operations");
    ch.unicast(ServerMessage::PendingDiscarded {
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
    });
}
