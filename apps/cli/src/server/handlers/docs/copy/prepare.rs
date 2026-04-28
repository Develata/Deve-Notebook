//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#internal-path-normalization

use super::super::copy::normalize_copy_dest_path;
use super::super::node_target::resolve_node_target;
use super::super::{checked_exists, errors, validate_file_path, validate_folder_path};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path};
use deve_core::models::{DocId, NodeKind};
use deve_core::protocol::doc_file_op_errors as path_err;
use std::{path::PathBuf, sync::Arc};
pub(super) struct CopyPaths {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub dst_repo_path: String,
    pub kind: NodeKind,
    pub src_doc_id: Option<DocId>,
}

pub(super) fn prepare_copy_paths(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src_path: &str,
    dest_path: &str,
    scope_nonce: Option<u64>,
) -> Option<CopyPaths> {
    let src = match resolve_node_target(state, scope, src_path) {
        Ok(Some(src)) => src,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ch,
                format!("Source not found: {}", src_path),
                scope_nonce,
            );
            return None;
        }
        Err(err) => {
            errors::classified_failure_scoped(ch, err.to_string(), scope_nonce);
            return None;
        }
    };
    let dst_repo_path = normalize_copy_dest_path(src.kind, dest_path);
    if src.repo_path == dst_repo_path {
        errors::request_failed_scoped(ch, path_err::DESTINATION_MUST_DIFFER, scope_nonce);
        return None;
    }
    let dst = match local_repo_path(state, scope, &dst_repo_path) {
        Ok(path) => path,
        Err(err) => {
            errors::classified_failure_scoped(ch, err.to_string(), scope_nonce);
            return None;
        }
    };
    let dst_exists = match checked_exists(&dst, "copy destination") {
        Ok(exists) => exists,
        Err(err) => {
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check copy destination: {}", err),
                scope_nonce,
            );
            return None;
        }
    };
    if dst_exists {
        errors::storage_conflict_scoped(
            ch,
            format!("Destination exists: {}", dst_repo_path),
            scope_nonce,
        );
        return None;
    }
    if !(if src.kind == NodeKind::Dir {
        validate_folder_path(&dst_repo_path, ch, scope_nonce)
    } else {
        validate_file_path(&dst_repo_path, ch, scope_nonce)
    }) {
        return None;
    }
    let mut src_exists = match checked_exists(&src.abs_path, "copy source projection") {
        Ok(exists) => exists,
        Err(err) => {
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check source projection: {}", err),
                scope_nonce,
            );
            return None;
        }
    };
    if src.kind == NodeKind::Dir && !src_exists {
        if let Err(err) = state
            .sync_manager
            .rebuild_projection_local_repo(&scope.repo_name)
        {
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rebuild source projection: {}", err),
                scope_nonce,
            );
            return None;
        }
        src_exists = match checked_exists(&src.abs_path, "rebuilt copy source projection") {
            Ok(exists) => exists,
            Err(err) => {
                errors::classified_failure_scoped(
                    ch,
                    format!("Failed to recheck source projection: {}", err),
                    scope_nonce,
                );
                return None;
            }
        };
    }
    if src.kind == NodeKind::Dir && !src_exists {
        errors::storage_not_found_scoped(
            ch,
            format!("Source projection missing: {}", src_path),
            scope_nonce,
        );
        return None;
    }
    Some(CopyPaths {
        src: src.abs_path,
        dst,
        dst_repo_path,
        kind: src.kind,
        src_doc_id: src.doc_id,
    })
}
