use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

use super::listing_scope::{
    browser_scope_nonce, clear_local_unbound_runtime_binding, precheck_remote_unbound_scope,
    send_listing_error,
};

/// 处理 ListShadows 请求 - 返回影子库列表 (远程分支)
pub async fn handle_list_shadows(
    state: &Arc<AppState>,
    ch: &DualChannel,
    mut session: Option<&mut WsSession>,
    request_id: Option<String>,
) {
    let scope_nonce = browser_scope_nonce(session.as_deref());
    if let Some(session) = session.as_deref_mut() {
        clear_local_unbound_runtime_binding(session);
        if precheck_remote_unbound_scope(state, ch, session, scope_nonce) {
            return;
        }
        if (session.active_repo.is_some()
            || session.active_repo_id.is_some()
            || session.has_runtime_scope_binding())
            && let Err(error) = resolve_session_repo_and_sync(state, session)
        {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(error), scope_nonce);
            return;
        }
    }
    match state.repo.list_switchable_shadows_on_disk() {
        Ok(peers) => {
            let self_peer = session
                .as_deref()
                .filter(|session| session.is_browser_session())
                .and_then(|session| session.authenticated_peer_id.clone());
            let shadows: Vec<String> = peers
                .into_iter()
                .filter(|peer: &PeerId| Some(peer.clone()) != self_peer)
                .map(|peer: PeerId| peer.to_string())
                .collect();
            ch.unicast(ServerMessage::ShadowList {
                request_id,
                scope_nonce,
                shadows,
            });
        }
        Err(e) => {
            tracing::error!("Failed to list shadow repos: {:?}", e);
            send_listing_error(
                ch,
                format!("Failed to list shadow repos: {}", e),
                scope_nonce,
            );
        }
    }
}
