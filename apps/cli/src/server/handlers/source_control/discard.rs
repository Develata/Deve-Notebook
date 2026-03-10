//! # 放弃工作区偏差
//!
//! Discard 只恢复工作区，不改写 Ledger 历史。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub async fn handle_discard_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let selector = super::service::selector_from_scope(&scope);
    let normalized = deve_core::utils::path::to_forward_slash(&path);

    match super::service::discard_pending(state.repo.as_ref(), &selector, &normalized) {
        Ok(_) => {
            tracing::info!("Discard pending workspace change: {}", normalized);
            ch.unicast(ServerMessage::DiscardAck {
                path: normalized.clone(),
            });
            super::changes::handle_get_changes(state, ch, session).await;
        }
        Err(e) => {
            tracing::error!("Failed to discard {}: {:?}", normalized, e);
            super::errors::send_ws(ch, e);
        }
    }
}
