// apps/cli/src/server/handlers/listing.rs
//! # 列表查询处理器
//!
//! 处理各类列表查询请求: ListDocs, ListShadows, ListRepos

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
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
    match state.repo.list_shadows_on_disk() {
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
                scope_nonce: session.map(WsSession::scope_nonce),
                shadows,
            });
        }
        Err(e) => {
            tracing::error!("Failed to list shadow repos: {:?}", e);
            send_listing_error(ch, format!("Failed to list shadow repos: {}", e));
        }
    }
}

/// 处理 ListRepos 请求 - 返回当前分支下的仓库列表
pub async fn handle_list_repos(
    state: &Arc<AppState>,
    ch: &DualChannel,
    active_branch: Option<&PeerId>,
    request_id: Option<String>,
    scope_nonce: Option<u64>,
) {
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
            send_listing_error(ch, format!("Failed to list repos: {}", e));
        }
    }
}

fn send_listing_error(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    ch.send_protocol_error(ServerError::with_detail(
        classify_listing_error(&detail),
        detail,
    ));
}

fn classify_listing_error(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "database already open",
            "cannot acquire lock",
            "db locked",
            "database is locked",
        ],
    ) {
        return ServerErrorCode::StorageDbLocked;
    }
    if contains_any(
        &lower,
        &[
            "remote session lost repo name",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "ambiguous local repository selector",
            "local repo not found for uuid",
        ],
    ) {
        return ServerErrorCode::ScRepoContextInvalid;
    }
    ServerErrorCode::RequestFailed
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
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
}
