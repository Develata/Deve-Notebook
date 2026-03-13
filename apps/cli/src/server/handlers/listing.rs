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
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list shadow repos: {}", e),
            ));
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
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list repos: {}", e),
            ));
        }
    }
}
