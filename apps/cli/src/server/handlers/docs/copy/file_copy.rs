use super::super::errors;
use super::prepare::CopyPaths;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::file_register::{
    broadcast_file_tree_update, create_file_from_content,
};
use crate::server::repo_scope::ResolvedRepo;
use deve_core::state;
use std::sync::Arc;

pub(super) fn copy_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    paths: &CopyPaths,
    src_path: &str,
    dest_path: &str,
) -> bool {
    let Some(doc_id) = paths.src_doc_id else {
        errors::storage_not_found(ch, format!("Source doc missing: {}", src_path));
        return false;
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
            tracing::error!("读取复制源失败 {}: {:?}", src_path, err);
            errors::storage_persist_failed(ch, format!("Failed to load copy source: {}", err));
            return false;
        }
    };
    let doc_id = match create_file_from_content(state, scope, dest_path, &content, "local_copy") {
        Ok(doc_id) => doc_id,
        Err(err) => {
            tracing::error!("复制文档注册失败 {}: {:?}", dest_path, err);
            errors::storage_persist_failed(ch, format!("Failed to register copied file: {}", err));
            return false;
        }
    };
    tracing::info!("已复制 {} -> {} (DocId: {})", src_path, dest_path, doc_id);
    broadcast_file_tree_update(state, ch, scope, doc_id);
    true
}
