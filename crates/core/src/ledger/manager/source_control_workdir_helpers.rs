//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/projection#projection-contract
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/index#internal-path-normalization
//!
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::pending_fs;
use crate::state::reconstruct_content;
use crate::utils::fs::checked_exists;
use anyhow::Result;
use std::path::Path;

use super::projection_cleanup::drop_unanchored_projection_path;

pub(super) fn rebuild_doc_projection(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
) -> Result<String> {
    let ops = repo.get_local_ops_in_local_repo(repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    Ok(reconstruct_content(&entries))
}

pub(super) fn discard_added(repo: &RepoManager, repo_name: &str, path: &str) -> Result<()> {
    repo.ensure_untracked_pending_add_path_in_local_repo(repo_name, path)?;
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    if workspace_path_exists(
        &file_path,
        &format!(
            "Failed to stat workspace path while discarding added file {}",
            path
        ),
    )? {
        repo.record_projection_delete(repo_name, path);
        if let Err(err) = std::fs::remove_file(&file_path) {
            repo.clear_projection_guard(repo_name, path);
            return Err(err.into());
        }
    }
    repo.run_on_local_repo(repo_name, |db| {
        pending_fs::remove(db, path)?;
        drop_unanchored_projection_path(db, path)?;
        Ok(())
    })
}

pub(super) fn discard_tracked_add(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
    doc_id: DocId,
) -> Result<()> {
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    if workspace_path_exists(
        &file_path,
        &format!(
            "Failed to stat workspace path while discarding tracked add {}",
            path
        ),
    )? {
        repo.record_projection_delete(repo_name, path);
        if let Err(err) = std::fs::remove_file(&file_path) {
            repo.clear_projection_guard(repo_name, path);
            return Err(err.into());
        }
    }
    let canonical_path = repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
        .map(|meta| meta.path)
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", doc_id))?;
    restore_doc_projection_at_path(repo, repo_name, doc_id, &canonical_path)?;
    repo.clear_pending_for_doc_in_local_repo(repo_name, doc_id, path)
}

pub(super) fn restore_doc_projection_at_path(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
    path: &str,
) -> Result<()> {
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    let content = rebuild_doc_projection(repo, repo_name, doc_id)?;
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    repo.record_projection_write(repo_name, path, &content);
    if let Err(err) = std::fs::write(&file_path, &content) {
        repo.clear_projection_guard(repo_name, path);
        return Err(err.into());
    }
    repo.bind_workspace_inode_in_local_repo(repo_name, path, doc_id)
}

pub(super) fn workspace_path_exists(path: &Path, context: &str) -> Result<bool> {
    checked_exists(path, context)
}
