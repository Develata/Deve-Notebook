//! plan_ref:
//!   - 04_storage#projection-contract

use super::{SnapshotPolicy, SyncManager, rebuild};
use crate::models::DocId;
use crate::utils::fs::checked_exists;
use anyhow::{Result, anyhow};
use tracing::{info, warn};

pub(super) fn persist_doc(sync: &SyncManager, repo_name: &str, doc_id: DocId) -> Result<()> {
    let Some(path_str) = sync
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
        .map(|meta| meta.path)
    else {
        return Err(anyhow!(
            "Tracked document projection missing for {} in repo {} while persisting projection",
            doc_id,
            repo_name
        ));
    };
    persist_doc_at_path(sync, repo_name, doc_id, &path_str)
}

pub(super) fn persist_doc_at_path(
    sync: &SyncManager,
    repo_name: &str,
    doc_id: DocId,
    path: &str,
) -> Result<()> {
    let file_path = sync.repo.local_repo_workspace_path(repo_name, path)?;
    let rebuilt = rebuild::rebuild_local_doc_in_repo(&sync.repo, repo_name, doc_id)?;
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    sync.repo
        .record_projection_write(repo_name, path, &rebuilt.content);
    if let Err(err) = std::fs::write(&file_path, &rebuilt.content) {
        sync.repo.clear_projection_guard(repo_name, path);
        return Err(err.into());
    }
    sync.repo
        .bind_workspace_inode_in_local_repo(repo_name, path, doc_id)?;
    info!("SyncManager: Persisted doc {} to {:?}", doc_id, file_path);
    let delta = rebuilt.max_seq.saturating_sub(rebuilt.base_seq);
    let policy = SnapshotPolicy::default();
    let doc_len = rebuilt.content.encode_utf16().count();
    if rebuilt.max_seq > 0
        && policy.should_snapshot(doc_len, delta, 0)
        && let Err(e) = sync.repo.save_snapshot_in_local_repo(
            repo_name,
            doc_id,
            rebuilt.max_seq,
            &rebuilt.content,
        )
    {
        warn!(
            "SyncManager: Failed to save snapshot for {}: {:?}",
            doc_id, e
        );
    }
    Ok(())
}

pub(super) fn remove_projection_path(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
) -> Result<()> {
    let file_path = sync.repo.local_repo_workspace_path(repo_name, path)?;
    if !checked_exists(&file_path, "workspace path while removing projection")? {
        return Ok(());
    }
    sync.repo.record_projection_delete(repo_name, path);
    if let Err(err) = std::fs::remove_file(file_path) {
        sync.repo.clear_projection_guard(repo_name, path);
        return Err(err.into());
    }
    Ok(())
}
