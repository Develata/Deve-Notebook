// apps/cli/src/server/handlers/docs/rename.rs
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/watcher#watcher-contract
//!   - 03_storage/index#internal-path-normalization
//!
//! # 重命名/移动文档处理器

use super::errors;
use super::node_target::resolve_node_target;
use super::rename_dir::handle_dir_rename;
use super::rename_file::handle_file_rename;
use super::{
    checked_exists, normalize_repo_path_input, resolve_local_write_scope, validate_file_path,
    validate_folder_path,
};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::local_repo_path;
use crate::server::session::WsSession;
use deve_core::models::NodeKind;
use deve_core::protocol::doc_file_op_errors as path_err;
use std::sync::Arc;

pub async fn handle_rename_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    old_path: String,
    new_path: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(scope) = resolve_local_write_scope(state, ch, session, scope_nonce) else {
        return;
    };

    let Some(old_path) = normalize_repo_path_input(&old_path) else {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return;
    };
    let Some(new_path) = normalize_repo_path_input(&new_path) else {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return;
    };
    if !validate_file_path(&old_path, ch, scope_nonce) {
        return;
    }
    let gate = state.repo_mutation_gate();
    let admission = match gate.admit_mounted_repo(scope.repo_id) {
        Ok(admission) => admission,
        Err(error) => {
            errors::server_error_scoped(ch, error.server_error(), scope_nonce);
            return;
        }
    };

    let src = match resolve_node_target(state, &scope, &old_path) {
        Ok(Some(target)) => target,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ch,
                format!("Source not found: {}", old_path),
                scope_nonce,
            );
            return;
        }
        Err(err) => {
            errors::classified_failure_scoped(ch, err.to_string(), scope_nonce);
            return;
        }
    };

    // 1. 确保目标路径以 .md 结尾 (如果源是 .md)
    let mut dst_name = new_path;
    if src.kind == NodeKind::File && !dst_name.ends_with(".md") {
        dst_name.push_str(".md");
    }

    // 2. 路径校验
    let valid = if src.kind == NodeKind::Dir {
        validate_folder_path(&dst_name, ch, scope_nonce)
    } else {
        validate_file_path(&dst_name, ch, scope_nonce)
    };
    if !valid {
        return;
    }

    if src.repo_path == dst_name {
        errors::request_failed_scoped(ch, path_err::DESTINATION_MUST_DIFFER, scope_nonce);
        return;
    }

    let dst = match local_repo_path(state, &scope, &dst_name) {
        Ok(path) => path,
        Err(err) => {
            errors::classified_failure_scoped(ch, err.to_string(), scope_nonce);
            return;
        }
    };

    match checked_exists(&dst, "rename destination") {
        Ok(true) => {
            tracing::error!("重命名失败: 目标已存在: {:?}", dst);
            errors::storage_conflict_scoped(
                ch,
                format!("Destination exists: {}", dst_name),
                scope_nonce,
            );
            return;
        }
        Ok(false) => {}
        Err(err) => {
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check rename destination: {}", err),
                scope_nonce,
            );
            return;
        }
    }

    // 3. 执行重命名
    if src.kind == NodeKind::File {
        handle_file_rename(
            state,
            ch,
            session,
            &scope,
            admission,
            &src.repo_path,
            &dst_name,
        )
        .await;
        return;
    }
    handle_dir_rename(
        state,
        ch,
        session,
        &scope,
        admission,
        &src.repo_path,
        &dst_name,
    )
    .await;
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
