use crate::api::WsService;
use crate::storage::identity::{note_handshake, save_repo_vector};
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::super::effects_msg;
use super::super::effects_sc;
use super::super::state::CoreSignals;

pub fn handle_sync_hello(
    peer_id: PeerId,
    repo_id: String,
    vector: VersionVector,
    signals: CoreSignals,
) {
    effects_msg::handle_sync_hello(peer_id, vector.clone(), signals.set_peers);
    let matches_current =
        signals.current_repo_id.get_untracked().as_deref() == Some(repo_id.as_str());
    if matches_current {
        signals.set_handshake_ready.set(true);
    }
    spawn_local(async move {
        let vector_json = serde_json::to_string(&vector).unwrap_or_default();
        let _ = save_repo_vector(&repo_id, &vector_json).await;
        let _ = note_handshake(&repo_id).await;
    });
}

pub fn handle_sc_or_remaining<F>(
    msg: ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
    schedule_refresh: &F,
) where
    F: Fn(),
{
    if !effects_sc::handle_sc_message(
        &msg,
        signals.set_staged_changes,
        signals.set_unstaged_changes,
        signals.set_commit_history,
        signals.set_diff_content,
        signals.set_commit_diff_result,
        signals.current_repo_id,
        schedule_refresh,
        ws,
    ) {
        effects_msg::handle_remaining(msg, signals.set_system_metrics);
    }
}
