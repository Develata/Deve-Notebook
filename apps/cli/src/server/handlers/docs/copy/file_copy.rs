use super::super::errors;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::file_register::{
    broadcast_file_tree_update, register_file_from_disk,
};
use crate::server::repo_scope::ResolvedRepo;
use std::path::Path;
use std::sync::Arc;

pub(super) fn copy_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src: &Path,
    dst: &Path,
    src_path: &str,
    dest_path: &str,
) -> bool {
    if let Err(e) = std::fs::copy(src, dst) {
        tracing::error!("复制失败 {} -> {:?}: {:?}", src_path, dst, e);
        errors::storage_persist_failed(ch, format!("Copy failed: {}", e));
        return false;
    }

    let doc_id = match register_file_from_disk(state, scope, dst, dest_path, "local_copy") {
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
