// apps/cli/src/server/handlers/listing.rs
//! # 列表查询处理器
//!
//! 处理各类列表查询请求: ListDocs, ListShadows, ListRepos

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
    active_repo: Option<&String>,
) {
    let docs = if let Some(peer_id) = active_branch {
        // Remote (Shadow)
        let repo_type = RepoType::Remote(peer_id.clone(), uuid::Uuid::nil()); // UUID handling to be improved later
        state.repo.list_docs(&repo_type)
    } else {
        // Local (Multi-Repo)
        state.repo.list_local_docs(active_repo.map(|s| s.as_str()))
    };

    match docs {
        Ok(docs_list) => {
            // 单播文档列表 (确保只有请求者收到，且用于当前 Repo 上下文)
            ch.unicast(ServerMessage::DocList { docs: docs_list });

            // 发送初始树结构 (Init Delta)
            if active_branch.is_none() {
                // TODO: TreeManager currently binds to main repo?
                // We should probably rely on DocList to rebuild tree in frontend for now
                // OR update TreeManager to support multi-repo.
                // Given the time constraint, we send Empty Tree to force frontend rebuild from DocList unless it's main repo?
                // Actually TreeManager init logic in `mod.rs` uses `list_docs`.
                // If we switched repo, TreeManager (global singleton) might still hold old tree.
                // This is a "Tenant" issue.
                // Critical: TreeManager should be per-repo or stateless?
                // TreeManager is `Arc<RwLock<TreeManager>>` in AppState.
                // If we switch repo, we should probably not use the shared TreeManager unless we reload it.
                // But TreeManager is shared across clients!
                // If client A is on Repo A, client B on Repo B.
                // We cannot share one TreeManager.
                // Solution: Send empty tree delta, force frontend to build from `DocList`.
                // Frontend `explorer.rs` has fallback: `if core_nodes.is_empty() { build_file_tree(docs) }`.
                use deve_core::tree::TreeDelta;
                ch.unicast(ServerMessage::TreeUpdate(TreeDelta::Init { roots: vec![] }));
            } else {
                use deve_core::tree::TreeDelta;
                ch.unicast(ServerMessage::TreeUpdate(TreeDelta::Init { roots: vec![] }));
            }
        }
        Err(e) => {
            tracing::error!("Failed to list docs: {:?}", e);
            ch.send_error(format!("Failed to list docs: {}", e));
        }
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
