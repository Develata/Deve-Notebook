use super::super::node_target::resolve_node_target;
use super::super::{errors, validate_file_path, validate_folder_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path};
use deve_core::models::{DocId, NodeKind};
use std::path::PathBuf;
use std::sync::Arc;

pub(super) struct CopyPaths {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub kind: NodeKind,
    pub src_doc_id: Option<DocId>,
}

pub(super) fn prepare_copy_paths(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src_path: &str,
    dest_path: &str,
) -> Option<CopyPaths> {
    let src = match resolve_node_target(state, scope, src_path) {
        Ok(Some(src)) => src,
        Ok(None) => {
            errors::storage_not_found(ch, format!("Source not found: {}", src_path));
            return None;
        }
        Err(err) => {
            errors::classified_failure(ch, err.to_string());
            return None;
        }
    };
    let dst = match local_repo_path(state, scope, dest_path) {
        Ok(path) => path,
        Err(err) => {
            errors::classified_failure(ch, err.to_string());
            return None;
        }
    };
    if dst.exists() {
        errors::storage_conflict(ch, format!("Destination exists: {}", dest_path));
        return None;
    }
    let valid = if src.kind == NodeKind::Dir {
        validate_folder_path(dest_path, ch)
    } else {
        validate_file_path(dest_path, ch)
    };
    if !valid {
        return None;
    }
    if src.kind == NodeKind::Dir
        && !src.abs_path.exists()
        && let Err(err) = state
            .sync_manager
            .rebuild_projection_local_repo(&scope.repo_name)
    {
        errors::storage_persist_failed(ch, format!("Failed to rebuild source projection: {}", err));
        return None;
    }
    if src.kind == NodeKind::Dir && !src.abs_path.exists() {
        errors::storage_not_found(ch, format!("Source projection missing: {}", src_path));
        return None;
    }
    Some(CopyPaths {
        src: src.abs_path,
        dst,
        kind: src.kind,
        src_doc_id: src.doc_id,
    })
}
