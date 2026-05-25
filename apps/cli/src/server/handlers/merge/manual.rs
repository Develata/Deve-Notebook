//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Manual merge sync-mode handlers.

use super::errors;
use super::manual_support::{sync_mode_label, with_engine, with_engine_mut};
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
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(repo_id) = resolve_read_repo_id(state, ch, session, scope_nonce) else {
        return;
    };
    let Some(mode) = with_engine(state, ch, repo_id, scope_nonce, |engine| {
        sync_mode_label(engine.sync_mode())
    }) else {
        return;
    };
    ch.unicast(ServerMessage::SyncModeStatus {
        request_id: Some(request_id),
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
        mode,
    });
}

pub(super) async fn handle_set_sync_mode(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    mode: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(repo_id) = resolve_write_repo_id(state, ch, session, scope_nonce) else {
        return;
    };
    let new_mode = match mode.to_lowercase().as_str() {
        "auto" => SyncMode::Auto,
        "manual" => SyncMode::Manual,
        _ => {
            return errors::request_failed(ch, format!("Invalid sync mode: {}", mode), scope_nonce);
        }
    };
    if with_engine_mut(state, ch, repo_id, scope_nonce, |engine| {
        engine.set_sync_mode(new_mode);
    })
    .is_none()
    {
        return;
    };
    tracing::info!("SetSyncMode: {:?}", new_mode);
    ch.unicast(ServerMessage::SyncModeStatus {
        request_id: None,
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
        mode: sync_mode_label(new_mode),
    });
}
