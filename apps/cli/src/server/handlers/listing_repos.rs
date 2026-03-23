use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

use super::{
    clear_local_unbound_runtime_binding, precheck_remote_unbound_scope, send_listing_error,
};

/// 处理 ListRepos 请求 - 返回当前分支下的仓库列表
pub async fn handle_list_repos(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
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
    let active_branch = session.active_branch.as_ref();
    match state.repo.list_repos(active_branch) {
        Ok(repos) => {
            ch.unicast(ServerMessage::RepoList {
                request_id,
                branch: active_branch.map(ToString::to_string),
                scope_nonce,
                repos,
            });
        }
        Err(e) => {
            tracing::error!("Failed to list repos: {:?}", e);
            send_listing_error(ch, format!("Failed to list repos: {}", e), scope_nonce);
        }
    }
}
