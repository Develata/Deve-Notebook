//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract

use super::checked_exists;
use crate::server::AppState;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path};
use anyhow::Result;
use deve_core::ledger::reconcile;
use deve_core::models::{DocId, StructureOp};
use deve_core::state;
use std::sync::Arc;

pub(super) fn create_file_from_content(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    rel_path: &str,
    content: &str,
    peer_label: &str,
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
        None,
        peer_label,
    )?;
    if !content.is_empty() {
        reconcile::append_patch_in_local_repo(
            state.repo.as_ref(),
            &scope.repo_name,
            doc_id,
            peer_label,
            &state::compute_diff("", content),
        )?;
    }
    state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)?;
    Ok((doc_id, ops))
}
