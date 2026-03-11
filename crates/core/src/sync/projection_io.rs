use super::{SnapshotPolicy, SyncManager, rebuild};
use crate::models::DocId;
use anyhow::Result;
use tracing::{info, warn};

pub(super) fn persist_doc(sync: &SyncManager, repo_name: &str, doc_id: DocId) -> Result<()> {
    if let Some(path_str) = sync
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
        .map(|meta| meta.path)
    {
        let file_path = sync.repo.local_repo_workspace_path(repo_name, &path_str)?;
        let rebuilt = rebuild::rebuild_local_doc_in_repo(&sync.repo, repo_name, doc_id)?;
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let relative_path = sync
            .repo
            .local_repo_workspace_relative(repo_name, &path_str);
        sync.persist_guard.record(&relative_path, &rebuilt.content);
        if let Err(err) = std::fs::write(&file_path, &rebuilt.content) {
            sync.persist_guard.clear(&relative_path);
            return Err(err.into());
        }
        sync.repo
            .bind_workspace_inode_in_local_repo(repo_name, &path_str, doc_id)?;
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
    }
    Ok(())
}

pub(super) fn remove_projection_path(
    sync: &SyncManager,
    repo_name: &str,
    path: &str,
) -> Result<()> {
    let file_path = sync.repo.local_repo_workspace_path(repo_name, path)?;
    if !file_path.exists() {
        return Ok(());
    }
    let relative_path = sync.repo.local_repo_workspace_relative(repo_name, path);
    sync.persist_guard.record_delete(&relative_path);
    if let Err(err) = std::fs::remove_file(file_path) {
        sync.persist_guard.clear(&relative_path);
        return Err(err.into());
    }
    Ok(())
}
