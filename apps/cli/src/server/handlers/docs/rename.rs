// apps/cli/src/server/handlers/docs/rename.rs
//! # 重命名/移动文档处理器

use super::errors;
use super::node_target::resolve_node_target;
use super::rename_dir::handle_dir_rename;
use super::rename_file::handle_file_rename;
use super::{validate_file_path, validate_folder_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{local_repo_path, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
use deve_core::models::NodeKind;
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
    session: &mut WsSession,
    old_path: String,
    new_path: String,
) {
    if session.is_readonly() {
        tracing::debug!("Rename rejected: session is readonly (remote branch)");
        errors::remote_branch_readonly(ch);
        return;
    }
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            errors::request_failed(ch, err.to_string());
            return;
        }
    };

    let src = match resolve_node_target(state, &scope, &old_path) {
        Ok(Some(target)) => target,
        Ok(None) => {
            errors::storage_not_found(ch, format!("Source not found: {}", old_path));
            return;
        }
        Err(err) => {
            errors::request_failed(ch, err.to_string());
            return;
        }
    };

    // 1. 确保目标路径以 .md 结尾 (如果源是 .md)
    let mut dst_name = new_path.clone();
    if src.kind == NodeKind::File && !dst_name.ends_with(".md") {
        dst_name.push_str(".md");
    }

    // 2. 路径校验
    let valid = if src.kind == NodeKind::Dir {
        validate_folder_path(&dst_name, ch)
    } else {
        validate_file_path(&dst_name, ch)
    };
    if !valid {
        return;
    }

    if old_path == dst_name {
        tracing::info!("重命名忽略: 路径未变化: {}", old_path);
        return;
    }

    let dst = match local_repo_path(state, &scope, &dst_name) {
        Ok(path) => path,
        Err(err) => {
            errors::request_failed(ch, err.to_string());
            return;
        }
    };

    if dst.exists() {
        tracing::error!("重命名失败: 目标已存在: {:?}", dst);
        errors::storage_conflict(ch, format!("Destination exists: {}", dst_name));
        return;
    }

    // 3. 执行重命名
    if src.kind == NodeKind::File {
        handle_file_rename(state, ch, session, &scope, &old_path, &dst_name).await;
        return;
    }
    handle_dir_rename(state, ch, session, &scope, &old_path, &dst_name).await;
}

/// 处理移动文档请求 (委托给 rename)
pub async fn handle_move_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    src_path: String,
    dest_path: String,
) {
    handle_rename_doc(state, ch, session, src_path, dest_path).await;
}
