//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/index#internal-path-normalization
//!   - 03_storage/watcher#watcher-contract
//!
//! 复制文档处理器入口。

mod dir_copy;
mod file_copy;
mod prepare;
mod register;

#[cfg(test)]
mod path_validation_tests;

use super::copy_utils::copy_dir_assets_only;
use super::errors;
use super::node_helpers::broadcast_local_projection_refresh;
use super::notify_fs_refresh;
use super::{normalize_repo_path_input, resolve_local_write_scope, validate_file_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::NodeKind;
use deve_core::protocol::doc_file_op_errors as path_err;
use prepare::prepare_copy_paths;
use register::CopyRegisterCtx;
use std::sync::Arc;

pub async fn handle_copy_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    src_path: String,
    dest_path: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(scope) = resolve_local_write_scope(state, ch, session, scope_nonce) else {
        return;
    };

    let Some(src_path) = normalize_repo_path_input(&src_path) else {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return;
    };
    let Some(dest_path) = normalize_repo_path_input(&dest_path) else {
        errors::request_failed_scoped(ch, path_err::INVALID_EMPTY_PATH, scope_nonce);
        return;
    };
    if !validate_file_path(&src_path, ch, scope_nonce) {
        return;
    }

    let paths = match prepare_copy_paths(state, ch, &scope, &src_path, &dest_path, scope_nonce) {
        Some(paths) => paths,
        None => return,
    };
    let dst_repo_path = paths.dst_repo_path.as_str();

    let copied = if paths.kind == deve_core::models::NodeKind::Dir {
        dir_copy::copy_dir(
            CopyRegisterCtx {
                state,
                ch,
                scope: &scope,
                scope_nonce,
            },
            &paths.src,
            &paths.dst,
            &src_path,
            dst_repo_path,
        )
    } else {
        file_copy::copy_file(
            CopyRegisterCtx {
                state,
                ch,
                scope: &scope,
                scope_nonce,
            },
            &paths,
            &src_path,
            dst_repo_path,
        )
    };
    if !copied {
        return;
    }

    if let Err(e) = broadcast_local_projection_refresh(state, ch, session, &scope) {
        tracing::error!("复制后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed_scoped(
            ch,
            format!("Failed to refresh copied docs view: {}", e),
            scope_nonce,
        );
        return;
    }
    notify_fs_refresh(ch, scope.repo_id, dst_repo_path, "copied");
}

fn copy_dir_on_disk(
    ch: &DualChannel,
    src: &std::path::Path,
    dst: &std::path::Path,
    src_path: &str,
    scope_nonce: Option<u64>,
) -> bool {
    if let Err(e) = copy_dir_assets_only(src, dst) {
        tracing::error!("目录复制失败 {} -> {:?}: {:?}", src_path, dst, e);
        errors::storage_persist_failed_scoped(
            ch,
            format!("Directory copy failed: {}", e),
            scope_nonce,
        );
        return false;
    }
    true
}

pub(super) fn normalize_copy_dest_path(kind: NodeKind, dest_path: &str) -> String {
    if kind == NodeKind::File && !dest_path.ends_with(".md") {
        format!("{}.md", dest_path)
    } else {
        dest_path.to_string()
    }
}
