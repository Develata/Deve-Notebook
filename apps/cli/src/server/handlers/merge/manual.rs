use super::scope::require_bound_repo_id;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::config::SyncMode;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;
use deve_core::sync::engine::SyncEngine;
use std::sync::Arc;

fn sync_mode_label(mode: SyncMode) -> String {
    if matches!(mode, SyncMode::Auto) {
        "auto"
    } else {
        "manual"
    }
    .to_string()
}

fn load_engine(state: &Arc<AppState>, ch: &DualChannel, repo_id: RepoId) -> Option<SyncEngine> {
    state.sync_engine.get_or_create(repo_id).or_else(|| {
        ch.send_error("Failed to get or create sync engine".to_string());
        None
    })
}

pub(super) async fn handle_get_sync_mode(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
) {
    let Some(repo_id) = require_bound_repo_id(ch, session) else {
        return;
    };
    let Some(engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    ch.unicast(ServerMessage::SyncModeStatus {
        mode: sync_mode_label(engine.sync_mode()),
    });
}

pub(super) async fn handle_set_sync_mode(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    mode: String,
) {
    let Some(repo_id) = require_bound_repo_id(ch, session) else {
        return;
    };
    let new_mode = match mode.to_lowercase().as_str() {
        "auto" => SyncMode::Auto,
        "manual" => SyncMode::Manual,
        _ => return ch.send_error(format!("Invalid sync mode: {}", mode)),
    };
    let Some(mut engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    engine.set_sync_mode(new_mode);
    tracing::info!("SetSyncMode: {:?}", new_mode);
    ch.unicast(ServerMessage::SyncModeStatus {
        mode: sync_mode_label(new_mode),
    });
}

pub(super) async fn handle_get_pending_ops(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
) {
    let Some(repo_id) = require_bound_repo_id(ch, session) else {
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
        count: pending_count as u32,
        previews,
    });
}

pub(super) async fn handle_confirm_merge(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
) {
    let Some(repo_id) = require_bound_repo_id(ch, session) else {
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
                merged_count: count as u32,
            });
        }
        Err(e) => {
            tracing::error!("Merge failed: {:?}", e);
            ch.send_error(format!("Merge failed: {}", e));
        }
    }
}

pub(super) async fn handle_discard_pending(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
) {
    let Some(repo_id) = require_bound_repo_id(ch, session) else {
        return;
    };
    let Some(mut engine) = load_engine(state, ch, repo_id) else {
        return;
    };
    engine.clear_pending();
    tracing::info!("Discarded all pending operations");
    ch.unicast(ServerMessage::PendingDiscarded);
}
