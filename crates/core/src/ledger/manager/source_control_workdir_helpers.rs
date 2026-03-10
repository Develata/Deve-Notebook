use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::pending_fs;
use crate::state::reconstruct_content;
use anyhow::Result;

use super::structure_projection::drop_transient_file_path;

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
    let file_path = repo.local_repo_workspace_path(repo_name, path)?;
    if file_path.exists() {
        std::fs::remove_file(&file_path)?;
    }
    repo.run_on_local_repo(repo_name, |db| {
        pending_fs::remove(db, path)?;
        drop_transient_file_path(db, path)?;
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
    if file_path.exists() {
        std::fs::remove_file(&file_path)?;
    }
    let canonical_path = repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
        .map(|meta| meta.path)
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", doc_id))?;
    let canonical_abs = repo.local_repo_workspace_path(repo_name, &canonical_path)?;
    if let Some(parent) = canonical_abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        canonical_abs,
        rebuild_doc_projection(repo, repo_name, doc_id)?,
    )?;
    repo.run_on_local_repo(repo_name, |db| clear_pending_for_doc(db, doc_id, path))
}

pub(super) fn clear_pending_for_doc(db: &redb::Database, doc_id: DocId, path: &str) -> Result<()> {
    let mut paths = vec![path.to_string()];
    for entry in pending_fs::list_for_doc(db, doc_id)? {
        if entry.doc_id == Some(doc_id) && !paths.iter().any(|item| item == &entry.path) {
            paths.push(entry.path);
        }
    }
    for path in paths {
        pending_fs::remove(db, &path)?;
    }
    Ok(())
}
