// apps/cli/src/server/handlers/docs/copy.rs
//! # 复制文档处理器
//!
//! 支持单文件和目录的复制操作。

use super::copy_utils::{collect_dirs, collect_md_files, copy_dir_recursive};
use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::{broadcast_dir_chain, broadcast_parent_dirs};
use crate::server::handlers::listing::handle_list_docs;
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use deve_core::models::NodeId;
use deve_core::protocol::ServerMessage;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理复制文档请求
///
/// **流程**:
/// 1. 校验源存在性、目标不存在
/// 2. 校验目标路径深度
/// 3. 执行文件系统复制 (支持目录递归)
/// 4. 在 Ledger 中为新文件创建 DocId (批量)
/// 5. 更新 TreeManager 并广播 TreeDelta
/// 6. 广播更新后的文档列表
pub async fn handle_copy_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    src_path: String,
    dest_path: String,
) {
    // 只读模式检查
    if session.is_readonly() {
        tracing::debug!("Copy ignored: session is readonly (remote branch)");
        return;
    }

    let src = join_normalized(&state.vault_path, &src_path);
    let dst = join_normalized(&state.vault_path, &dest_path);

    // 1. 源检查
    if !src.exists() {
        tracing::error!("复制失败: 源不存在: {:?}", src);
        ch.send_error(format!("Source not found: {}", src_path));
        return;
    }

    // 2. 目标检查
    if dst.exists() {
        tracing::error!("复制失败: 目标已存在: {:?}", dst);
        ch.send_error(format!("Destination exists: {}", dest_path));
        return;
    }

    // 3. 路径校验
    if !validate_path(&dest_path, ch) {
        return;
    }

    // 4. 确保目标父目录存在
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // 5. 执行复制并更新 TreeManager
    if src.is_dir() {
        // 目录递归复制
        if let Err(e) = copy_dir_recursive(&src, &dst) {
            tracing::error!("目录复制失败 {} -> {:?}: {:?}", src_path, dst, e);
            ch.send_error(format!("Directory copy failed: {}", e));
            return;
        }
        // 批量注册 Ledger 并更新 TreeManager
        register_and_broadcast_copied_docs(state, ch, &dst, &dest_path);
        if let Ok(report) = state
            .repo
            .run_on_local_repo(state.repo.local_repo_name(), |db| {
                deve_core::ledger::node_check::check_node_consistency(db)
            })
            && !report.is_clean()
        {
            tracing::warn!(
                "Node consistency after copy: missing={} orphan={}",
                report.missing_nodes.len(),
                report.orphan_nodes.len()
            );
        }
        tracing::info!("已复制目录 {} -> {}", src_path, dest_path);
    } else {
        // 单文件复制
        if let Err(e) = std::fs::copy(&src, &dst) {
            tracing::error!("复制失败 {} -> {:?}: {:?}", src_path, dst, e);
            ch.send_error(format!("Copy failed: {}", e));
            return;
        }
        // 注册单个文档并更新 TreeManager
        if let Ok(doc_id) = state.repo.create_docid(&dest_path) {
            tracing::info!("已复制 {} -> {} (DocId: {})", src_path, dest_path, doc_id);
            let node_id = NodeId::from_doc_id(doc_id);
            if let Ok(meta) = state
                .repo
                .run_on_local_repo(state.repo.local_repo_name(), |db| {
                    node_meta::get_node_meta(db, node_id)
                        .and_then(|m| m.ok_or_else(|| anyhow!("File node meta missing")))
                })
            {
                if let Err(e) = broadcast_parent_dirs(state, ch, meta.parent_id) {
                    tracing::error!("广播父目录失败: {:?}", e);
                }
                let delta = state.tree_manager.write().unwrap().add_file(
                    node_id,
                    meta.path.clone(),
                    meta.parent_id,
                    meta.name.clone(),
                    doc_id,
                );
                ch.broadcast(ServerMessage::TreeUpdate(delta));
            }
        } else {
            tracing::error!("Ledger 注册复制文档失败");
        }
    }

    handle_list_docs(state, ch, session).await;
}

/// 批量注册复制目录下的所有 `.md` 文件到 Ledger，并更新 TreeManager
fn register_and_broadcast_copied_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    dst: &std::path::Path,
    dest_path: &str,
) {
    let base = &state.vault_path;

    if let Ok(dirs) = collect_dirs(dst, base) {
        for dir_path in dirs {
            let created = state
                .repo
                .run_on_local_repo(state.repo.local_repo_name(), |db| {
                    if let Some(existing) = node_meta::get_node_id(db, &dir_path)? {
                        let meta = node_meta::get_node_meta(db, existing)?
                            .ok_or_else(|| anyhow!("Node meta missing"))?;
                        if meta.kind != deve_core::models::NodeKind::Dir {
                            return Err(anyhow!("Path is not a directory: {}", dir_path));
                        }
                        return Ok(Some((existing, meta)));
                    }
                    let node_id = node_meta::create_dir_node(db, &dir_path)?;
                    let meta = node_meta::get_node_meta(db, node_id)?
                        .ok_or_else(|| anyhow!("Dir node meta missing"))?;
                    Ok(Some((node_id, meta)))
                });

            match created {
                Ok(Some((node_id, _meta))) => {
                    if let Err(e) = broadcast_dir_chain(state, ch, node_id) {
                        tracing::error!("广播目录链失败: {:?}", e);
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::error!("目录节点创建失败: {:?}", e);
                    ch.send_error(format!("Dir node creation failed: {}", e));
                    return;
                }
            }
        }
    }

    match collect_md_files(dst, base) {
        Ok(files) => {
            let count = files.len();
            for rel_path in files {
                if let Ok(doc_id) = state.repo.create_docid(&rel_path) {
                    tracing::debug!("注册复制文档: {} (DocId: {})", rel_path, doc_id);
                    let node_id = NodeId::from_doc_id(doc_id);
                    if let Ok(meta) =
                        state
                            .repo
                            .run_on_local_repo(state.repo.local_repo_name(), |db| {
                                node_meta::get_node_meta(db, node_id).and_then(|m| {
                                    m.ok_or_else(|| anyhow!("File node meta missing"))
                                })
                            })
                    {
                        if let Err(e) = broadcast_parent_dirs(state, ch, meta.parent_id) {
                            tracing::error!("广播父目录失败: {:?}", e);
                        }
                        let delta = state.tree_manager.write().unwrap().add_file(
                            node_id,
                            meta.path.clone(),
                            meta.parent_id,
                            meta.name.clone(),
                            doc_id,
                        );
                        ch.broadcast(ServerMessage::TreeUpdate(delta));
                    }
                } else {
                    tracing::warn!("Ledger 注册失败: {}", rel_path);
                }
            }
            tracing::info!("目录复制完成: {} 下注册 {} 个文档", dest_path, count);
        }
        Err(e) => {
            tracing::error!("收集 .md 文件失败: {:?}", e);
        }
    }
}
