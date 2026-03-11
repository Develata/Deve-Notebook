//! 复制文档处理器入口。

mod dir_copy;
mod file_copy;
mod prepare;
mod register;

use super::copy_utils::copy_dir_assets_only;
use super::errors;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::resolve_session_repo;
use crate::server::session::WsSession;
use prepare::prepare_copy_paths;
use std::sync::Arc;

pub async fn handle_copy_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    src_path: String,
    dest_path: String,
) {
    if session.is_readonly() {
        tracing::debug!("Copy ignored: session is readonly (remote branch)");
        return;
    }
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => return errors::request_failed(ch, err.to_string()),
    };
    let paths = match prepare_copy_paths(state, ch, &scope, &src_path, &dest_path) {
        Some(paths) => paths,
        None => return,
    };

    let copied = if paths.kind == deve_core::models::NodeKind::Dir {
        dir_copy::copy_dir(
            state, ch, &scope, &paths.src, &paths.dst, &src_path, &dest_path,
        )
    } else {
        file_copy::copy_file(state, ch, &scope, &paths, &src_path, &dest_path)
    };
    if !copied {
        return;
    }

    handle_list_docs(state, ch, session).await;
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
