//! # Listing Handlers
//!
//! 处理各类列表查询请求:
//! - List Docs
//! - List Shadows
//! - List Repos

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use deve_core::models::{PeerId, RepoType}; // Added RepoType
use deve_core::ledger::listing::RepoListing; // Added RepoListing trait
use crate::server::AppState;

/// 处理 ListDocs 请求 - 列出 Vault 中的所有文档
pub async fn handle_list_docs(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    active_branch: Option<&PeerId>,
) {
     let repo_type = match active_branch {
         Some(peer_id) => RepoType::Remote(peer_id.clone(), uuid::Uuid::nil()), // Default RepoId
         None => RepoType::Local(uuid::Uuid::nil()),
     };

     if let Ok(docs) = state.repo.list_docs(&repo_type) {
         let msg = ServerMessage::DocList { docs };
         let _ = tx.send(msg);
     }
}

/// 处理 ListShadows 请求 - 返回影子库列表 (远程分支)
pub async fn handle_list_shadows(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    tracing::info!("Handling ListShadows request. Remotes dir: {:?}", state.repo.remotes_dir());
    match state.repo.list_shadows_on_disk() {
        Ok(peers) => {
            let shadows: Vec<String> = peers.iter()
                .map(|p| p.to_string())
                .collect();
            tracing::info!("Found {} shadow repos: {:?}", shadows.len(), shadows);
            let _ = tx.send(ServerMessage::ShadowList { shadows });
        }
        Err(e) => {
            tracing::error!("Failed to list shadow repos: {:?}", e);
            let _ = tx.send(ServerMessage::ShadowList { shadows: vec![] });
        }
    }
}

/// 处理 ListRepos 请求 - 返回当前分支下的仓库列表
pub async fn handle_list_repos(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    active_branch: Option<&PeerId>,
) {
    match state.repo.list_repos(active_branch) {
        Ok(repos) => {
            let _ = tx.send(ServerMessage::RepoList { repos });
        }
        Err(e) => {
            tracing::error!("Failed to list repos: {:?}", e);
            let _ = tx.send(ServerMessage::RepoList { repos: vec![] });
        }
    }
}
