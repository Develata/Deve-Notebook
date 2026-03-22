// apps/cli/src/server/handlers/listing.rs
//! # 列表查询处理器
//!
//! 处理各类列表查询请求: ListDocs, ListShadows, ListRepos

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::resolve_session_repo_and_sync;
use crate::server::repo_scope::{map_repo_scope_error, stale_unbound_remote_scope_detail};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use anyhow::anyhow;
use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

#[path = "listing_docs.rs"]
mod listing_docs;

pub use listing_docs::handle_list_docs;

/// 处理 ListShadows 请求 - 返回影子库列表 (远程分支)
pub async fn handle_list_shadows(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: Option<&WsSession>,
    request_id: Option<String>,
) {
    let scope_nonce = browser_scope_nonce(session);
    match state.repo.list_switchable_shadows_on_disk() {
        Ok(peers) => {
            let self_peer = session
                .filter(|session| session.is_browser_session())
                .and_then(|session| session.authenticated_peer_id.clone());
            let shadows: Vec<String> = peers
                .into_iter()
                .filter(|peer| Some(peer.clone()) != self_peer)
                .map(|peer| peer.to_string())
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

/// 处理 ListRepos 请求 - 返回当前分支下的仓库列表
pub async fn handle_list_repos(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
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

pub(super) fn precheck_remote_unbound_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> bool {
    let Some(branch) = session.active_branch.as_ref().cloned() else {
        return false;
    };
    if session.active_repo.is_some() || session.active_repo_id.is_some() {
        return false;
    }
    if let Err(error) = shadow_scope::map_remote_branch_availability(state, &branch) {
        shadow_scope::clear_stale_remote_branch(session);
        ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
        return true;
    }
    if session.has_runtime_scope_binding() {
        session.clear_active_db();
        session.clear_sync_binding();
        ch.send_protocol_error_with_scope_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                stale_unbound_remote_scope_detail(&branch),
            ),
            scope_nonce,
        );
        return true;
    }
    false
}

fn send_listing_error(ch: &DualChannel, detail: impl Into<String>, scope_nonce: Option<u64>) {
    let detail = detail.into();
    ch.send_protocol_error_with_scope_nonce(
        ServerError::with_detail(classify_listing_error(&detail), detail),
        scope_nonce,
    );
}

fn browser_scope_nonce(session: Option<&WsSession>) -> Option<u64> {
    session
        .filter(|session| session.is_browser_session())
        .map(WsSession::scope_nonce)
}

fn classify_listing_error(detail: &str) -> ServerErrorCode {
    map_repo_scope_error(anyhow!(detail.to_string())).code
}

#[cfg(test)]
mod tests {
    use super::classify_listing_error;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_locked_listing_db_as_storage_db_locked() {
        assert_eq!(
            classify_listing_error("Database already open. Cannot acquire lock."),
            ServerErrorCode::StorageDbLocked
        );
    }

    #[test]
    fn classifies_listing_scope_drift_as_repo_context_invalid() {
        assert_eq!(
            classify_listing_error("Remote session lost repo name for current branch"),
            ServerErrorCode::ScRepoContextInvalid
        );
    }

    #[test]
    fn classifies_ambiguous_remote_selector_as_repo_context_invalid() {
        assert_eq!(
            classify_listing_error("Ambiguous remote repository selector: shadow-wiki"),
            ServerErrorCode::ScRepoContextInvalid
        );
    }

    #[test]
    fn classifies_missing_repo_selection_as_sync_repo_unbound() {
        assert_eq!(
            classify_listing_error("Active repository not selected: multiple local repos exist"),
            ServerErrorCode::SyncRepoUnbound
        );
    }

    #[test]
    fn classifies_remote_workspace_access_as_repo_context_invalid() {
        assert_eq!(
            classify_listing_error("Local workspace root requested on remote branch: wiki"),
            ServerErrorCode::ScRepoContextInvalid
        );
    }

    #[test]
    fn classifies_broken_shadow_listing_as_storage_persist_failed() {
        assert_eq!(
            classify_listing_error("Broken shadow repo notes for peer peer-a while listing repos"),
            ServerErrorCode::StoragePersistFailed
        );
    }
}
