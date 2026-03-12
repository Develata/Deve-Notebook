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
    if should_accept_sync_hello(
        signals.current_repo_id.get_untracked(),
        signals.active_branch.get_untracked(),
        &repo_id,
    ) {
        signals.set_handshake_ready.set(true);
    }
    spawn_local(async move {
        match serde_json::to_string(&vector) {
            Ok(vector_json) => {
                let _ = save_repo_vector(&repo_id, &vector_json).await;
            }
            Err(err) => leptos::logging::warn!("保存 repo 向量失败: {}", err),
        }
        let _ = note_handshake(&repo_id).await;
    });
}

fn should_accept_sync_hello(
    current_repo_id: Option<String>,
    active_branch: Option<PeerId>,
    repo_id: &str,
) -> bool {
    active_branch.is_none() && current_repo_id.as_deref() == Some(repo_id)
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

#[cfg(test)]
mod tests {
    use super::should_accept_sync_hello;
    use deve_core::models::PeerId;

    #[test]
    fn ignores_sync_hello_while_viewing_remote_branch() {
        let repo_id = uuid::Uuid::new_v4().to_string();
        assert!(!should_accept_sync_hello(
            Some(repo_id.clone()),
            Some(PeerId::new("peer-a")),
            &repo_id,
        ));
    }
}
