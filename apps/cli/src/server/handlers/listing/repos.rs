//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-catalog-contract

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::repo_list::repo_list_message;
use crate::server::repo_scope::resolve_session_repo_and_sync;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::scope::{
    clear_local_unbound_runtime_binding, map_listing_repo_scope_error,
    precheck_remote_unbound_scope, send_listing_error,
};

/// 处理 ListRepos 请求 - 返回当前分支下的仓库列表
pub async fn handle_list_repos(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
) {
    if !session.is_browser_session() {
        ch.send_protocol_error_with_scope_nonce(
            ServerError::new(ServerErrorCode::ScRepoContextInvalid),
            None,
        );
        return;
    }
    let scope_nonce = Some(session.scope_nonce());
    clear_local_unbound_runtime_binding(state, session);
    if precheck_remote_unbound_scope(state, ch, session, scope_nonce) {
        return;
    }
    if (session.active_repo.is_some()
        || session.active_repo_id.is_some()
        || session.has_runtime_scope_binding())
        && let Err(error) = resolve_session_repo_and_sync(state, session)
    {
        ch.send_protocol_error_with_scope_nonce(map_listing_repo_scope_error(error), scope_nonce);
        return;
    }
    let active_branch = session.active_branch.as_ref();
    match repo_list_message(state, request_id, active_branch, scope_nonce) {
        Ok(message) => ch.unicast(message),
        Err(e) => {
            tracing::error!("Failed to list repos: {:?}", e);
            send_listing_error(ch, format!("Failed to list repos: {}", e), scope_nonce);
        }
    }
}
