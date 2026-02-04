// apps/cli/src/server/handlers/docs/rename.rs
//! # 重命名/移动文档处理器

use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_parent_dirs;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::node_meta;
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
    session: &WsSession,
    old_path: String,
    new_path: String,
) {
    // 只读模式检查: 静默忽略重命名请求
    // TODO: Frontend will hide rename buttons when readonly
    if session.is_readonly() {
        tracing::debug!("Rename ignored: session is readonly (remote branch)");
        return;
    }

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

    if dst.exists() {
        tracing::error!("重命名失败: 目标已存在: {:?}", dst);
        ch.send_error(format!("Destination exists: {}", dst_name));
        return;
    }

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
            if let Ok((node_id, meta)) =
                state
                    .repo
                    .run_on_local_repo(state.repo.local_repo_name(), |db| {
                        let node_id = node_meta::get_node_id(db, &dst_name)?
                            .ok_or_else(|| anyhow!("Node not found: {}", dst_name))?;
                        let meta = node_meta::get_node_meta(db, node_id)?
                            .ok_or_else(|| anyhow!("Node meta missing"))?;
                        Ok((node_id, meta))
                    })
            {
                if let Err(e) = broadcast_parent_dirs(state, ch, meta.parent_id) {
                    tracing::error!("广播父目录失败: {:?}", e);
                }
                let delta = state.tree_manager.write().unwrap().update_node(
                    node_id,
                    meta.parent_id,
                    meta.name.clone(),
                    meta.path.clone(),
                );
                ch.broadcast(ServerMessage::TreeUpdate(delta));
            }

            // 6. 刷新文档列表
            handle_list_docs(state, ch, session).await;
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
    session: &WsSession,
    src_path: String,
    dest_path: String,
) {
    handle_rename_doc(state, ch, session, src_path, dest_path).await;
}
