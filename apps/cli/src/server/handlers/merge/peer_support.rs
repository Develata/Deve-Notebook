//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Peer merge support helpers for repository-scoped sessions.

use crate::server::repo_scope::{ResolvedRepo, resolve_local_counterpart_repo};
use crate::server::{AppState, channel::DualChannel};
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::DOCID_TO_PATH;
use deve_core::models::DocId;
use std::sync::Arc;

use super::errors;

pub(super) fn resolve_doc_path(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_name: &str,
    doc_id: DocId,
    scope_nonce: Option<u64>,
) -> Option<String> {
    match state
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
    {
        Ok(Some(meta)) => Some(meta.path),
        Err(e) => {
            errors::classified_failure(
                ch,
                format!("Failed to resolve merged doc path: {}", e),
                scope_nonce,
            );
            None
        }
        Ok(None) => {
            match legacy_doc_path(state.repo.as_ref(), repo_name, doc_id) {
                Ok(Some(path)) => errors::classified_failure(
                    ch,
                    format!(
                        "Tracked document projection missing for legacy-mapped doc: {}",
                        path
                    ),
                    scope_nonce,
                ),
                Ok(None) => errors::storage_not_found(
                    ch,
                    "Doc path not found for merged document",
                    scope_nonce,
                ),
                Err(e) => errors::classified_failure(
                    ch,
                    format!("Failed to resolve merged doc path: {}", e),
                    scope_nonce,
                ),
            }
            None
        }
    }
}

fn legacy_doc_path(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
) -> anyhow::Result<Option<String>> {
    repo.run_on_local_repo(repo_name, |db| {
        let read = db.begin_read()?;
        let table = read.open_table(DOCID_TO_PATH)?;
        Ok(table
            .get(doc_id.as_u128())?
            .map(|path| path.value().to_string()))
    })
}

pub(super) fn resolve_local_merge_scope(
    state: &Arc<AppState>,
    scope: ResolvedRepo,
    ch: &DualChannel,
    scope_nonce: Option<u64>,
) -> Option<ResolvedRepo> {
    match resolve_local_counterpart_repo(state, &scope) {
        Ok(Some(local_scope)) => Some(local_scope),
        Ok(None) => {
            errors::storage_not_found(
                ch,
                "No local repository matched the active remote repository",
                scope_nonce,
            );
            None
        }
        Err(err) => {
            errors::classified_failure(
                ch,
                format!("Failed to resolve local merge scope: {}", err),
                scope_nonce,
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "peer_support_test.rs"]
mod tests;
