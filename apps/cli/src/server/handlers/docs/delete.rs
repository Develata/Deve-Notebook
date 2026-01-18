// apps/cli/src/server/handlers/docs/delete.rs
//! # 删除文档处理器

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理删除文档请求
///
/// **流程**:
/// 1. 判断目标是文件还是目录
/// 2. 执行文件系统删除
/// 3. 从 Ledger 中移除记录
/// 4. 广播更新后的文档列表
pub async fn handle_delete_doc(state: &Arc<AppState>, ch: &DualChannel, path: String) {
    tracing::info!("handle_delete_doc: path={}", path);
    let target = join_normalized(&state.vault_path, &path);
    let is_dir = target.is_dir();

    // 1. 执行文件系统删除
    if target.exists() {
        if is_dir {
            if let Err(e) = std::fs::remove_dir_all(&target) {
                tracing::error!("删除目录失败 {}: {:?}", path, e);
            }
        } else if let Err(e) = std::fs::remove_file(&target) {
            tracing::error!("删除文件失败 {}: {:?}", path, e);
        }
    } else {
        tracing::warn!("待删除文件不存在: {:?}", target);
    }

    // 2. 更新 Ledger
    if is_dir {
        match state.repo.delete_folder(&path) {
            Ok(count) => tracing::info!("已从 Ledger 删除 {} 个文档 (文件夹: {})", count, path),
            Err(e) => tracing::error!("Ledger 文件夹删除失败: {:?}", e),
        }
    } else if let Err(e) = state.repo.delete_doc(&path) {
        tracing::error!("Ledger 文档删除失败: {:?}", e);
    }

    // 3. 广播列表
    handle_list_docs(state, ch, None).await;
}
