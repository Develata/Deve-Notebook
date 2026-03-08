//! 复制文档处理器入口。

mod dir_copy;
mod file_copy;
mod register;

use super::copy_utils::copy_dir_recursive;
use super::{notify_fs_refresh, validate_file_path, validate_folder_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path, resolve_session_repo};
use crate::server::session::WsSession;
use std::path::PathBuf;
use std::sync::Arc;

struct CopyPaths {
    src: PathBuf,
    dst: PathBuf,
}

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
        Err(err) => return ch.send_error(err.to_string()),
    };
    let paths = match prepare_copy_paths(state, ch, &scope, &src_path, &dest_path) {
        Some(paths) => paths,
        None => return,
    };

    if paths.src.is_dir() {
        dir_copy::copy_dir(state, ch, &scope, &paths.dst, &src_path, &dest_path);
    } else {
        file_copy::copy_file(
            state, ch, &scope, &paths.src, &paths.dst, &src_path, &dest_path,
        );
    }

    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, &dest_path, "copied");
}

fn prepare_copy_paths(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src_path: &str,
    dest_path: &str,
) -> Option<CopyPaths> {
    let src = match local_repo_path(state, scope, src_path) {
        Ok(path) => path,
        Err(err) => {
            ch.send_error(err.to_string());
            return None;
        }
    };
    let dst = match local_repo_path(state, scope, dest_path) {
        Ok(path) => path,
        Err(err) => {
            ch.send_error(err.to_string());
            return None;
        }
    };
    if !src.exists() {
        tracing::error!("复制失败: 源不存在: {:?}", src);
        ch.send_error(format!("Source not found: {}", src_path));
        return None;
    }
    if dst.exists() {
        tracing::error!("复制失败: 目标已存在: {:?}", dst);
        ch.send_error(format!("Destination exists: {}", dest_path));
        return None;
    }
    let valid = if src.is_dir() {
        validate_folder_path(dest_path, ch)
    } else {
        validate_file_path(dest_path, ch)
    };
    if !valid {
        return None;
    }
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    Some(CopyPaths { src, dst })
}

fn copy_dir_on_disk(
    ch: &DualChannel,
    src: &std::path::Path,
    dst: &std::path::Path,
    src_path: &str,
) -> bool {
    if let Err(e) = copy_dir_recursive(src, dst) {
        tracing::error!("目录复制失败 {} -> {:?}: {:?}", src_path, dst, e);
        ch.send_error(format!("Directory copy failed: {}", e));
        return false;
    }
    true
}
