//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 05_diff_logic#source-control-runtime

use super::{SyncManager, projection_io};
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeStatus, pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

pub(super) fn discard_pending_workdir(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
) -> Result<()> {
    let normalized = to_forward_slash(path);
    let pending = sync
        .repo
        .run_on_local_repo(repo_name, |db| pending_fs::get(db, &normalized))?;
    if let Some(entry) = pending {
        return discard_entry(sync, repo_name, normalized, entry);
    }

    let staged = sync
        .repo
        .run_on_local_repo(repo_name, |db| staging::get_staged(db, &normalized))?;
    if let Some(entry) = staged {
        return discard_staged_entry(sync, repo_name, normalized, entry);
    }

    anyhow::bail!("Path is not in pending_fs_ops or staging: {}", normalized);
}

pub(super) fn discard_pending_target_workdir(
    sync: &SyncManager,
    repo_name: &str,
    target: &ScPathTarget,
) -> Result<String> {
    let pending = sync
        .repo
        .run_on_local_repo(repo_name, |db| pending_fs::get_for_target(db, target))?;
    if let Some(entry) = pending {
        let normalized = to_forward_slash(&entry.path);
        discard_entry(sync, repo_name, normalized.clone(), entry)?;
        return Ok(normalized);
    }

    let staged = sync
        .repo
        .run_on_local_repo(repo_name, |db| staging::get_staged_for_target(db, target))?;
    let Some((path, entry)) = staged else {
        anyhow::bail!("Path is not in pending_fs_ops or staging: {}", target.path);
    };
    let normalized = to_forward_slash(&path);
    discard_staged_entry(sync, repo_name, normalized.clone(), entry)?;
    Ok(normalized)
}

fn discard_entry(
    sync: &SyncManager,
    repo_name: &str,
    normalized: String,
    entry: pending_fs::PendingFsEntry,
) -> Result<()> {
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
            let canonical_path = canonical_doc_path(sync, repo_name, doc_id)?;
            if canonical_path != normalized {
                projection_io::remove_projection_path(sync, repo_name, &normalized)?;
            }
            restore_projection(sync, repo_name, doc_id, &canonical_path)?;
            sync.repo
                .clear_pending_for_doc_in_local_repo(repo_name, doc_id, &normalized)
        }
    }
}

fn discard_added(sync: &SyncManager, repo_name: &str, path: &str) -> Result<()> {
    sync.repo
        .ensure_untracked_pending_add_path_in_local_repo(repo_name, path)?;
    projection_io::remove_projection_path(sync, repo_name, path)?;
    sync.repo
        .discard_untracked_pending_add_in_local_repo(repo_name, path)
}

fn discard_staged_entry(
    sync: &SyncManager,
    repo_name: &str,
    normalized: String,
    entry: staging::StagedEntry,
) -> Result<()> {
    match entry.status {
        ChangeStatus::Added => match entry.doc_id {
            Some(doc_id) => discard_staged_tracked_add(sync, repo_name, &normalized, doc_id),
            None => discard_staged_added(sync, repo_name, &normalized),
        },
        ChangeStatus::Modified | ChangeStatus::Deleted | ChangeStatus::Renamed => {
            let doc_id = match entry.doc_id {
                Some(doc_id) => doc_id,
                None => sync
                    .repo
                    .resolve_workdir_doc_id_in_local_repo(repo_name, &normalized)?
                    .ok_or_else(|| anyhow::anyhow!("Document not found: {}", normalized))?,
            };
            let canonical_path = canonical_doc_path(sync, repo_name, doc_id)?;
            if canonical_path != normalized {
                projection_io::remove_projection_path(sync, repo_name, &normalized)?;
            }
            restore_projection(sync, repo_name, doc_id, &canonical_path)?;
            clear_staged_for_doc_or_path(sync, repo_name, Some(doc_id), &normalized)
        }
    }
}

fn discard_staged_added(sync: &SyncManager, repo_name: &str, path: &str) -> Result<()> {
    projection_io::remove_projection_path(sync, repo_name, path)?;
    clear_staged_for_doc_or_path(sync, repo_name, None, path)
}

fn discard_staged_tracked_add(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
    doc_id: crate::models::DocId,
) -> Result<()> {
    projection_io::remove_projection_path(sync, repo_name, path)?;
    let canonical_path = canonical_doc_path(sync, repo_name, doc_id)?;
    restore_projection(sync, repo_name, doc_id, &canonical_path)?;
    clear_staged_for_doc_or_path(sync, repo_name, Some(doc_id), path)
}

fn clear_staged_for_doc_or_path(
    sync: &SyncManager,
    repo_name: &str,
    doc_id: Option<crate::models::DocId>,
    path: &str,
) -> Result<()> {
    sync.repo.run_on_local_repo(repo_name, |db| {
        if let Some(doc_id) = doc_id {
            let staged_paths = staging::list_staged_entries_for_doc(db, doc_id)?
                .into_iter()
                .map(|(path, _)| path)
                .collect::<Vec<_>>();
            for staged_path in staged_paths {
                let _ = staging::take_staged(db, &staged_path)?;
            }
            return Ok(());
        }
        let _ = staging::take_staged(db, path)?;
        Ok(())
    })
}

fn discard_tracked_add(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
    doc_id: crate::models::DocId,
) -> Result<()> {
    projection_io::remove_projection_path(sync, repo_name, path)?;
    let canonical_path = canonical_doc_path(sync, repo_name, doc_id)?;
    restore_projection(sync, repo_name, doc_id, &canonical_path)?;
    sync.repo
        .clear_pending_for_doc_in_local_repo(repo_name, doc_id, path)
}

fn canonical_doc_path(
    sync: &SyncManager,
    repo_name: &str,
    doc_id: crate::models::DocId,
) -> Result<String> {
    sync.repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
        .map(|meta| meta.path)
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", doc_id))
}

fn restore_projection(
    sync: &SyncManager,
    repo_name: &str,
    doc_id: crate::models::DocId,
    path: &str,
) -> Result<()> {
    projection_io::persist_doc_at_path(sync, repo_name, doc_id, path)
}
