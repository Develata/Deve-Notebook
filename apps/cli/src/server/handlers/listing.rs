// apps/cli/src/server/handlers/listing.rs
//! # 列表查询处理器
//!
//! 处理各类列表查询请求: ListDocs, ListShadows, ListRepos

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 处理 ListDocs 请求 - 列出当前激活仓库中的所有文档
///
/// **逻辑**:
/// 优先使用 session 中锁定的 active_db 列出文档。
/// 这确保文件树与当前选中的 repo 保持一致。
/// 同时发送 RepoSwitched 通知前端当前仓库名称。
pub async fn handle_list_docs(state: &Arc<AppState>, ch: &DualChannel, session: &WsSession) {
    // 确定当前仓库名称
    let current_repo = session
        .active_repo
        .clone()
        .unwrap_or_else(|| state.repo.local_repo_name().to_string());

    // 发送 RepoSwitched 让前端知道当前仓库 (用于初始化及切换后同步)
    ch.unicast(ServerMessage::RepoSwitched {
        name: current_repo.clone(),
        uuid: String::new(), // TODO: Fetch UUID
    });

    // 使用 session 锁定的数据库，或回退到默认逻辑
    let docs = if let Some(handle) = session.get_active_db() {
        // 直接从锁定的数据库读取文档列表
        deve_core::ledger::metadata::list_docs(&handle.db)
    } else {
        // 回退: 使用 active_branch/active_repo 字符串
        if let Some(peer_id) = &session.active_branch {
            let repo_type = deve_core::models::RepoType::Remote(peer_id.clone(), uuid::Uuid::nil());
            state.repo.list_docs(&repo_type)
        } else {
            state.repo.list_local_docs(session.active_repo.as_deref())
        }
    };

    match docs {
        Ok(docs_list) => {
            tracing::info!(
                "ListDocs: Returning {} docs for repo '{}'",
                docs_list.len(),
                current_repo
            );
            // 单播文档列表 (确保只有请求者收到，且用于当前 Repo 上下文)
            ch.unicast(ServerMessage::DocList { docs: docs_list });

            // 发送空树结构，让前端从 DocList 重建
            // (TreeManager 是全局共享的，无法用于多租户场景)
            use deve_core::tree::TreeDelta;
            ch.unicast(ServerMessage::TreeUpdate(TreeDelta::Init { roots: vec![] }));
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
