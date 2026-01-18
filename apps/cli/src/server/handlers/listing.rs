// apps/cli/src/server/handlers/listing.rs
//! # 列表查询处理器
//!
//! 处理各类列表查询请求: ListDocs, ListShadows, ListRepos

use super::get_repo_id;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoType};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 处理 ListDocs 请求 - 列出 Vault 中的所有文档
pub async fn handle_list_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    active_branch: Option<&PeerId>,
) {
    let repo_type = match active_branch {
        Some(peer_id) => RepoType::Remote(peer_id.clone(), uuid::Uuid::nil()),
        None => RepoType::Local(get_repo_id(state)),
    };

    if let Ok(docs) = state.repo.list_docs(&repo_type) {
        // 广播文档列表 (其他标签页也需要更新)
        ch.broadcast(ServerMessage::DocList { docs });
    }
}

/// 处理 ListShadows 请求 - 返回影子库列表 (远程分支)
pub async fn handle_list_shadows(state: &Arc<AppState>, ch: &DualChannel) {
    tracing::info!(
        "Handling ListShadows request. Remotes dir: {:?}",
        state.repo.remotes_dir()
    );
    match state.repo.list_shadows_on_disk() {
        Ok(peers) => {
            let shadows: Vec<String> = peers.iter().map(|p| p.to_string()).collect();
            tracing::info!("Found {} shadow repos: {:?}", shadows.len(), shadows);
            // 单播给请求者
            ch.unicast(ServerMessage::ShadowList { shadows });
        }
        Err(e) => {
            tracing::error!("Failed to list shadow repos: {:?}", e);
            ch.unicast(ServerMessage::ShadowList { shadows: vec![] });
        }
    }
}

/// 处理 ListRepos 请求 - 返回当前分支下的仓库列表
pub async fn handle_list_repos(
    state: &Arc<AppState>,
    ch: &DualChannel,
    active_branch: Option<&PeerId>,
) {
    match state.repo.list_repos(active_branch) {
        Ok(repos) => {
            // 单播给请求者
            ch.unicast(ServerMessage::RepoList { repos });
        }
        Err(e) => {
            tracing::error!("Failed to list repos: {:?}", e);
            ch.unicast(ServerMessage::RepoList { repos: vec![] });
        }
    }
}
