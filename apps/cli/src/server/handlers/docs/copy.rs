//! 复制文档处理器入口。

mod dir_copy;
mod file_copy;
mod prepare;
mod register;

use super::copy_utils::copy_dir_assets_only;
use super::errors;
use super::node_helpers::broadcast_local_projection_refresh;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
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
    if session.is_readonly() {
        tracing::debug!("Copy rejected: session is readonly (remote branch)");
        errors::remote_branch_readonly_scoped(ch, scope_nonce);
        return;
    }
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            return;
        }
    };
    let paths = match prepare_copy_paths(state, ch, &scope, &src_path, &dest_path) {
        Some(paths) => paths,
        None => return,
    };

    let copied = if paths.kind == deve_core::models::NodeKind::Dir {
        dir_copy::copy_dir(
            CopyRegisterCtx {
                state,
                ch,
                scope: &scope,
            },
            &paths.src,
            &paths.dst,
            &src_path,
            &dest_path,
        )
    } else {
        file_copy::copy_file(
            CopyRegisterCtx {
                state,
                ch,
                scope: &scope,
            },
            &paths,
            &src_path,
            &dest_path,
        )
    };
    if !copied {
        return;
    }

    if let Err(e) = broadcast_local_projection_refresh(state, ch, session, &scope) {
        tracing::error!("复制后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed(ch, format!("Failed to refresh copied docs view: {}", e));
        return;
    }
    notify_fs_refresh(ch, scope.repo_id, &dest_path, "copied");
}

fn copy_dir_on_disk(
    ch: &DualChannel,
    src: &std::path::Path,
    dst: &std::path::Path,
    src_path: &str,
) -> bool {
    if let Err(e) = copy_dir_assets_only(src, dst) {
        tracing::error!("目录复制失败 {} -> {:?}: {:?}", src_path, dst, e);
        errors::storage_persist_failed(ch, format!("Directory copy failed: {}", e));
        return false;
    }
    true
}
