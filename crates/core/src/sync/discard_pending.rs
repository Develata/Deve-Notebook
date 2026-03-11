use super::{SyncManager, projection_io};
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
        ChangeStatus::Modified | ChangeStatus::Deleted | ChangeStatus::Renamed => {
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
    projection_io::remove_projection_path(sync, repo_name, path)?;
    sync.repo
        .discard_untracked_pending_add_in_local_repo(repo_name, path)
}

fn discard_tracked_add(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
    doc_id: crate::models::DocId,
) -> Result<()> {
    projection_io::remove_projection_path(sync, repo_name, path)?;
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
    projection_io::persist_doc_at_path(sync, repo_name, doc_id, path)
}
