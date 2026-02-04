// apps/cli/src/server/handlers/docs/delete.rs
//! # 删除文档处理器

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::session::WsSession;
use deve_core::ledger::node_meta;
use deve_core::protocol::ServerMessage;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理删除文档请求
///
/// **流程**:
/// 1. 判断目标是文件还是目录
/// 2. 执行文件系统删除
/// 3. 从 Ledger 中移除记录
/// 4. 更新 TreeManager 并广播 TreeDelta
pub async fn handle_delete_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    // 只读模式检查: 静默忽略删除请求
    // TODO: Frontend will hide delete buttons when readonly
    if session.is_readonly() {
        tracing::debug!("Delete ignored: session is readonly (remote branch)");
        return;
    }

    tracing::info!("handle_delete_doc: path={}", path);
    let target = join_normalized(&state.vault_path, &path);
    let is_dir = target.is_dir();

    // 1. 获取 NodeId (用于 TreeDelta)
    let node_id = state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            node_meta::get_node_id(db, &path)
        })
        .ok()
        .flatten();

    // 2. 执行文件系统删除
    if target.exists() {
        if is_dir {
            if let Err(e) = std::fs::remove_dir_all(&target) {
                tracing::error!("删除目录失败 {}: {:?}", path, e);
                ch.send_error(format!("Failed to delete directory: {}", e));
                return;
            }
        } else if let Err(e) = std::fs::remove_file(&target) {
            tracing::error!("删除文件失败 {}: {:?}", path, e);
            ch.send_error(format!("Failed to delete file: {}", e));
            return;
        }
    } else {
        tracing::warn!("待删除文件不存在: {:?}", target);
        ch.send_error("Target not found".to_string());
        return;
    }

    // 3. 更新 Ledger
    if is_dir {
        match state.repo.delete_folder(&path) {
            Ok(count) => tracing::info!("已从 Ledger 删除 {} 个文档 (文件夹: {})", count, path),
            Err(e) => tracing::error!("Ledger 文件夹删除失败: {:?}", e),
        }
    } else if let Err(e) = state.repo.delete_doc(&path) {
        tracing::error!("Ledger 文档删除失败: {:?}", e);
    }

    // 4. 更新 TreeManager 并广播 Delta
    if let Some(node_id) = node_id {
        let delta = state.tree_manager.write().unwrap().remove(node_id);
        ch.broadcast(ServerMessage::TreeUpdate(delta));
    }

    // 5. 刷新文档列表
    handle_list_docs(state, ch, session).await;
}
