use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::copy_utils::{collect_dirs, collect_md_files};
use crate::server::handlers::docs::file_register::{
    broadcast_file_tree_update, register_file_from_disk,
};
use crate::server::handlers::docs::node_helpers::broadcast_dir_chain;
use crate::server::repo_scope::{ResolvedRepo, local_repo_root};
use std::path::Path;
use std::sync::Arc;

pub(super) fn register_copied_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    dst: &Path,
    dest_path: &str,
) {
    let base = match local_repo_root(state, scope) {
        Ok(path) => path,
        Err(err) => {
            ch.send_error(err.to_string());
            return;
        }
    };
    register_dirs(state, ch, scope, dst, &base);
    register_files(state, ch, scope, dst, &base, dest_path);
}

fn register_dirs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    dst: &Path,
    base: &Path,
) {
    let Ok(dirs) = collect_dirs(dst, base) else {
        return;
    };
    for dir_path in dirs {
        match state.repo.apply_dir_create_structure_in_local_repo(
            &scope.repo_name,
            &dir_path,
            "local_copy",
        ) {
            Ok(node_id) => {
                if let Err(e) =
                    broadcast_dir_chain(state, ch, scope.repo_id, &scope.repo_name, node_id)
                {
                    tracing::error!("广播目录链失败: {:?}", e);
                }
            }
            Err(e) => {
                tracing::error!("目录节点创建失败: {:?}", e);
                ch.send_error(format!("Dir node creation failed: {}", e));
                return;
            }
        }
    }
}

fn register_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    dst: &Path,
    base: &Path,
    dest_path: &str,
) {
    match collect_md_files(dst, base) {
        Ok(files) => {
            let count = files.len();
            for rel_path in files {
                register_file(state, ch, scope, &rel_path);
            }
            tracing::info!("目录复制完成: {} 下注册 {} 个文档", dest_path, count);
        }
        Err(e) => tracing::error!("收集 .md 文件失败: {:?}", e),
    }
}

fn register_file(state: &Arc<AppState>, ch: &DualChannel, scope: &ResolvedRepo, rel_path: &str) {
    let disk_path = match local_repo_root(state, scope) {
        Ok(root) => root.join(rel_path),
        Err(err) => {
            ch.send_error(err.to_string());
            return;
        }
    };
    let Ok(doc_id) = register_file_from_disk(state, scope, &disk_path, rel_path, "local_copy")
    else {
        tracing::warn!("Ledger 注册失败: {}", rel_path);
        return;
    };
    tracing::debug!("注册复制文档: {} (DocId: {})", rel_path, doc_id);
    broadcast_file_tree_update(state, ch, scope, doc_id);
}
