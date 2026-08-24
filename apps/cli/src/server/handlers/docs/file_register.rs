//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract

use super::checked_exists;
use crate::server::AppState;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path};
use anyhow::Result;
use deve_core::ledger::reconcile;
use deve_core::models::{DocId, Op, StructureOp};
use deve_core::state;
use std::sync::Arc;

pub(super) fn create_file_from_content(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    rel_path: &str,
    content: &str,
    peer_label: &str,
    doc_id_hint: Option<DocId>,
) -> Result<(DocId, Vec<StructureOp>)> {
    let patch = state::compute_diff("", content)?;
    create_file_from_patch(
        state,
        scope,
        rel_path,
        content,
        &patch,
        peer_label,
        doc_id_hint,
    )
}

pub(super) fn create_file_from_patch(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    rel_path: &str,
    content: &str,
    patch: &[Op],
    peer_label: &str,
    doc_id_hint: Option<DocId>,
) -> Result<(DocId, Vec<StructureOp>)> {
    let path = local_repo_path(state, scope, rel_path)?;
    if checked_exists(&path, "file register target")? {
        anyhow::bail!("Target file already exists on disk: {}", rel_path);
    }
    if state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, rel_path)?
        .is_some()
    {
        anyhow::bail!("Target already tracked: {}", rel_path);
    }
    let (doc_id, ops) = state.repo.apply_file_structure_in_local_repo(
        &scope.repo_name,
        rel_path,
        doc_id_hint,
        peer_label,
    )?;
    if !patch.is_empty() {
        reconcile::append_patch_in_local_repo(
            state.repo.as_ref(),
            &scope.repo_name,
            doc_id,
            peer_label,
            patch,
        )?;
    }
    state
        .sync_manager
        .persist_prepared_doc_content_in_local_repo(&scope.repo_name, doc_id, content)?;
    Ok((doc_id, ops))
}
