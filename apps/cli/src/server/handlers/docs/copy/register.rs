use super::super::errors;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::copy_utils::{collect_dirs, collect_md_files};
use crate::server::handlers::docs::file_register::{
    broadcast_file_tree_update, create_file_from_content,
};
use crate::server::handlers::docs::node_helpers::broadcast_dir_chain;
use crate::server::repo_scope::{ResolvedRepo, local_repo_root};
use deve_core::state;
use std::path::Path;
use std::sync::Arc;

pub(super) fn register_copied_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let base = match local_repo_root(state, scope) {
        Ok(path) => path,
        Err(err) => {
            errors::request_failed(ch, err.to_string());
            return false;
        }
    };
    register_dirs(state, ch, scope, src, &base, src_path, dest_path)
        && register_files(state, ch, scope, src, &base, src_path, dest_path)
}

fn register_dirs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src: &Path,
    base: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let mut dirs = match collect_dirs(src, base) {
        Ok(dirs) => dirs,
        Err(err) => {
            tracing::error!("收集目录失败: {:?}", err);
            errors::storage_persist_failed(ch, format!("Failed to collect copied dirs: {}", err));
            return false;
        }
    };
    dirs.sort_by_key(|path| path.matches('/').count());
    for dir_path in dirs {
        let Some(dest_rel) = map_dest_rel(&dir_path, src_path, dest_path) else {
            errors::request_failed(ch, format!("Invalid copied dir path: {}", dir_path));
            return false;
        };
        match state.repo.apply_dir_create_structure_in_local_repo(
            &scope.repo_name,
            &dest_rel,
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
                errors::storage_persist_failed(ch, format!("Dir node creation failed: {}", e));
                return false;
            }
        }
    }
    true
}

fn register_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src: &Path,
    base: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    match collect_md_files(src, base) {
        Ok(files) => {
            let count = files.len();
            for rel_path in files {
                let Some(dest_rel) = map_dest_rel(&rel_path, src_path, dest_path) else {
                    errors::request_failed(ch, format!("Invalid copied path: {}", rel_path));
                    return false;
                };
                if !register_file(state, ch, scope, &rel_path, &dest_rel) {
                    return false;
                }
            }
            tracing::info!("目录复制完成: {} 下注册 {} 个文档", dest_path, count);
            true
        }
        Err(e) => {
            tracing::error!("收集 .md 文件失败: {:?}", e);
            errors::storage_persist_failed(ch, format!("Failed to collect copied files: {}", e));
            false
        }
    }
}

fn register_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src_rel: &str,
    dest_rel: &str,
) -> bool {
    let doc_id = match state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, src_rel)
    {
        Ok(Some(doc_id)) => doc_id,
        Ok(None) => {
            errors::storage_not_found(ch, format!("Source doc not tracked: {}", src_rel));
            return false;
        }
        Err(err) => {
            errors::request_failed(ch, format!("Failed to resolve copied source: {}", err));
            return false;
        }
    };
    let content = match state
        .repo
        .get_local_ops_in_local_repo(&scope.repo_name, doc_id)
    {
        Ok(ops) => {
            let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
            state::reconstruct_content(&entries)
        }
        Err(err) => {
            tracing::error!("加载复制源失败 {}: {:?}", src_rel, err);
            errors::storage_persist_failed(ch, format!("Failed to load copied file: {}", err));
            return false;
        }
    };
    let doc_id = match create_file_from_content(state, scope, dest_rel, &content, "local_copy") {
        Ok(doc_id) => doc_id,
        Err(err) => {
            tracing::error!("Ledger 注册失败 {}: {:?}", dest_rel, err);
            errors::storage_persist_failed(ch, format!("Failed to register copied file: {}", err));
            return false;
        }
    };
    tracing::debug!(
        "注册复制文档: {} -> {} (DocId: {})",
        src_rel,
        dest_rel,
        doc_id
    );
    broadcast_file_tree_update(state, ch, scope, doc_id);
    true
}

fn map_dest_rel(src_rel: &str, src_path: &str, dest_path: &str) -> Option<String> {
    let suffix = src_rel.strip_prefix(src_path)?;
    let trimmed = suffix.trim_start_matches('/');
    if trimmed.is_empty() {
        return Some(dest_path.to_string());
    }
    Some(format!("{}/{}", dest_path.trim_end_matches('/'), trimmed))
}
