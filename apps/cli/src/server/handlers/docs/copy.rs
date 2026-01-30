// apps/cli/src/server/handlers/docs/copy.rs
//! # 复制文档处理器
//!
//! 支持单文件和目录的复制操作。

use super::copy_utils::{collect_md_files, copy_dir_recursive};
use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::session::WsSession;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理复制文档请求
///
/// **流程**:
/// 1. 校验源存在性、目标不存在
/// 2. 校验目标路径深度
/// 3. 执行文件系统复制 (支持目录递归)
/// 4. 在 Ledger 中为新文件创建 DocId (批量)
/// 5. 广播更新后的文档列表
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

    // 5. 执行复制
    if src.is_dir() {
        // 目录递归复制
        if let Err(e) = copy_dir_recursive(&src, &dst) {
            tracing::error!("目录复制失败 {} -> {:?}: {:?}", src_path, dst, e);
            ch.send_error(format!("Directory copy failed: {}", e));
            return;
        }
        // 批量注册 Ledger
        register_copied_docs(state, &dst, &dest_path);
        tracing::info!("已复制目录 {} -> {}", src_path, dest_path);
    } else {
        // 单文件复制
        if let Err(e) = std::fs::copy(&src, &dst) {
            tracing::error!("复制失败 {} -> {:?}: {:?}", src_path, dst, e);
            ch.send_error(format!("Copy failed: {}", e));
            return;
        }
        // 注册单个文档
        if let Ok(doc_id) = state.repo.create_docid(&dest_path) {
            tracing::info!("已复制 {} -> {} (DocId: {})", src_path, dest_path, doc_id);
        } else {
            tracing::error!("Ledger 注册复制文档失败");
        }
    }

    handle_list_docs(state, ch, session).await;
}

/// 批量注册复制目录下的所有 `.md` 文件到 Ledger
fn register_copied_docs(state: &Arc<AppState>, dst: &std::path::Path, dest_path: &str) {
    // 计算相对路径基准
    let base = &state.vault_path;

    match collect_md_files(dst, base) {
        Ok(files) => {
            let count = files.len();
            for rel_path in files {
                if let Ok(doc_id) = state.repo.create_docid(&rel_path) {
                    tracing::debug!("注册复制文档: {} (DocId: {})", rel_path, doc_id);
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
