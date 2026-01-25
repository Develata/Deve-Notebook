// apps/cli/src/server/handlers/docs/copy.rs
//! # 复制文档处理器

use super::validate_path;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use deve_core::utils::path::join_normalized;
use std::sync::Arc;

/// 处理复制文档请求
///
/// **流程**:
/// 1. 校验源文件存在性、目标文件不存在
/// 2. 校验目标路径深度
/// 3. 执行文件系统复制
/// 4. 在 Ledger 中为新文件创建 DocId
/// 5. 广播更新后的文档列表
pub async fn handle_copy_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    src_path: String,
    dest_path: String,
) {
    let src = join_normalized(&state.vault_path, &src_path);
    let dst = join_normalized(&state.vault_path, &dest_path);

    // 1. 源文件检查
    if !src.exists() {
        tracing::error!("复制失败: 源不存在: {:?}", src);
        ch.send_error(format!("Source not found: {}", src_path));
        return;
    }

    // 2. 目标文件检查
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
        tracing::warn!("目录复制在 MVP 中暂不支持");
        ch.send_error("Directory copy not supported".to_string());
    } else if let Err(e) = std::fs::copy(&src, &dst) {
        tracing::error!("复制失败 {} -> {:?}: {:?}", src_path, dst, e);
        ch.send_error(format!("Copy failed: {}", e));
    } else if let Ok(doc_id) = state.repo.create_docid(&dest_path) {
        tracing::info!(
            "已复制 {} -> {} (新 DocId: {})",
            src_path,
            dest_path,
            doc_id
        );
        handle_list_docs(state, ch, None, None).await;
    } else {
        tracing::error!("Ledger 注册复制文档失败");
    }
}
