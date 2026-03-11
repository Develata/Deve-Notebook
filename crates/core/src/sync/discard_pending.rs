use super::{SyncManager, rebuild};
use crate::source_control::{ChangeStatus, pending_fs};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

pub(super) fn discard_pending_workdir(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
) -> Result<()> {
    let normalized = to_forward_slash(path);
    let entry = sync
        .repo
        .run_on_local_repo(repo_name, |db| pending_fs::get(db, &normalized))?
        .ok_or_else(|| anyhow::anyhow!("Path is not in pending_fs_ops: {}", normalized))?;
    match entry.change_type {
        ChangeStatus::Added => match entry.doc_id {
            Some(doc_id) => discard_tracked_add(sync, repo_name, &normalized, doc_id),
            None => discard_added(sync, repo_name, &normalized),
        },
        ChangeStatus::Modified | ChangeStatus::Deleted => {
            let doc_id = match entry.doc_id {
                Some(doc_id) => doc_id,
                None => sync
                    .repo
                    .resolve_workdir_doc_id_in_local_repo(repo_name, &normalized)?
                    .ok_or_else(|| anyhow::anyhow!("Document not found: {}", normalized))?,
            };
            restore_projection(sync, repo_name, doc_id, &normalized)?;
            sync.repo
                .clear_pending_for_doc_in_local_repo(repo_name, doc_id, &normalized)
        }
    }
}

fn discard_added(sync: &SyncManager, repo_name: &str, path: &str) -> Result<()> {
    let file_path = sync.repo.local_repo_workspace_path(repo_name, path)?;
    if file_path.exists() {
        std::fs::remove_file(file_path)?;
    }
    sync.repo
        .discard_untracked_pending_add_in_local_repo(repo_name, path)
}

fn discard_tracked_add(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
    doc_id: crate::models::DocId,
) -> Result<()> {
    let file_path = sync.repo.local_repo_workspace_path(repo_name, path)?;
    if file_path.exists() {
        std::fs::remove_file(file_path)?;
    }
    let canonical_path = sync
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
        .map(|meta| meta.path)
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", doc_id))?;
    restore_projection(sync, repo_name, doc_id, &canonical_path)?;
    sync.repo
        .clear_pending_for_doc_in_local_repo(repo_name, doc_id, path)
}

fn restore_projection(
    sync: &SyncManager,
    repo_name: &str,
    doc_id: crate::models::DocId,
    path: &str,
) -> Result<()> {
    let rebuilt = rebuild::rebuild_local_doc_in_repo(&sync.repo, repo_name, doc_id)?;
    let file_path = sync.repo.local_repo_workspace_path(repo_name, path)?;
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let relative_path = sync.repo.local_repo_workspace_relative(repo_name, path);
    sync.persist_guard.record(&relative_path, &rebuilt.content);
    if let Err(err) = std::fs::write(&file_path, &rebuilt.content) {
        sync.persist_guard.clear(&relative_path);
        return Err(err.into());
    }
    sync.repo
        .bind_workspace_inode_in_local_repo(repo_name, path, doc_id)
}
