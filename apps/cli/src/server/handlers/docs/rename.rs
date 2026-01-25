// apps/cli/src/server/handlers/docs/rename.rs
//! # 重命名/移动文档处理器

use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use deve_core::protocol::ServerMessage;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理重命名文档请求
///
/// **流程**:
/// 1. 校验目标路径
/// 2. 执行文件系统重命名
/// 3. 更新 Ledger 中的路径映射
/// 4. 更新 TreeManager 并广播 TreeDelta
pub async fn handle_rename_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    old_path: String,
    new_path: String,
) {
    let src = join_normalized(&state.vault_path, &old_path);

    // 1. 确保目标路径以 .md 结尾 (如果源是 .md)
    let mut dst_name = new_path.clone();
    if !dst_name.ends_with(".md") && old_path.ends_with(".md") {
        dst_name.push_str(".md");
    }

    // 2. 路径校验
    if !validate_path(&dst_name, ch) {
        return;
    }

    let dst = join_normalized(&state.vault_path, &dst_name);

    // 3. 执行重命名
    if src.exists() {
        // 确保目标父目录存在
        if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Err(e) = std::fs::rename(&src, &dst) {
            tracing::error!("重命名失败 {} -> {}: {:?}", old_path, dst_name, e);
            ch.send_error(format!("Failed to rename: {}", e));
        } else {
            tracing::info!("已重命名 {} -> {}", old_path, dst_name);

            // 4. 更新 Ledger
            if dst.is_dir() {
                if let Err(e) = state.repo.rename_folder(&old_path, &dst_name) {
                    tracing::error!("Ledger 文件夹重命名失败: {:?}", e);
                }
            } else if let Err(e) = state.repo.rename_doc(&old_path, &dst_name) {
                tracing::error!("Ledger 文档重命名失败: {:?}", e);
            }

            // 5. 更新 TreeManager 并广播 Delta
            let delta = state
                .tree_manager
                .write()
                .unwrap()
                .rename(&old_path, &dst_name);
            ch.broadcast(ServerMessage::TreeUpdate(delta));

            // 6. 兼容旧逻辑
            handle_list_docs(state, ch, None, None).await;
        }
    } else {
        tracing::warn!("重命名失败: 源不存在: {:?}", src);
        ch.send_error(format!("Source not found: {}", old_path));
    }
}

/// 处理移动文档请求 (委托给 rename)
pub async fn handle_move_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    src_path: String,
    dest_path: String,
) {
    handle_rename_doc(state, ch, src_path, dest_path).await;
}
