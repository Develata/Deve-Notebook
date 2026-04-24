//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
//! Source-control commit write helper.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn commit_with_ack(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    message: String,
    success_label: &str,
    error_label: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let selector = super::service::selector_from_scope(&scope);
    match super::service::commit_staged(state.repo.as_ref(), &selector, &message) {
        Ok(info) => {
            tracing::info!("{}: {} - {}", success_label, info.id, info.message);
            ch.broadcast(ServerMessage::CommitAck {
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                commit_id: info.id,
                timestamp: info.timestamp,
            });
        }
        Err(e) => {
            tracing::error!("{}: {:?}", error_label, e);
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}
