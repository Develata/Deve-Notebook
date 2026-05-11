//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 04_storage#watcher-contract
//!
//! # 放弃工作区偏差
//!
//! Discard 只恢复工作区，不改写 Ledger 历史。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ScPathTarget;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub async fn handle_discard_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    target: ScPathTarget,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_writable_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let selector = super::service::selector_from_scope(&scope);
    match super::local_discard::discard_via_sync_manager(state, &selector, &target) {
        Ok(path) => {
            tracing::info!("Discard pending workspace change: {}", path);
            ch.unicast(ServerMessage::DiscardAck {
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                path: path.clone(),
            });
            super::changes::handle_get_changes(state, ch, session, None).await;
        }
        Err(e) => {
            tracing::error!("Failed to discard {}: {:?}", target.path, e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}
