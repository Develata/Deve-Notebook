use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use crate::source_control::diff;
use crate::source_control::{pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

impl RepoManager {
    pub fn stage_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| {
            let exact_path = to_forward_slash(&target.path);
            let exact = pending_fs::get(db, &exact_path)?.filter(|entry| {
                target
                    .doc_id
                    .map(|doc_id| entry.doc_id == Some(doc_id))
                    .unwrap_or(entry.doc_id.is_none())
            });
            let entry = if let Some(entry) = exact {
                pending_fs::remove(db, &entry.path)?;
                entry
            } else if let Some(entry) = pending_fs::take_for_target(db, target)? {
                entry
            } else {
                anyhow::bail!("Path is not in pending_fs_ops: {}", target.path);
            };
            crate::ledger::source_control::stage_pending_entry(db, &entry)
        })
    }

    pub fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path = self
            .run_on_local_repo(repo_name, |db| pending_fs::get_for_target(db, target))?
            .map(|entry| entry.path)
            .ok_or_else(|| anyhow::anyhow!("Path is not in pending_fs_ops: {}", target.path))?;
        self.discard_pending_workdir_in_local_repo(repo_name, &path)
    }

    pub fn unstage_file_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| {
            let Some((path, staged)) = staging::take_staged_for_target(db, target)? else {
                anyhow::bail!("Path is not staged: {}", target.path);
            };
            pending_fs::upsert(
                db,
                &pending_fs::PendingFsEntry {
                    path,
                    renamed_from: staged.renamed_from,
                    doc_id: staged.doc_id,
                    change_type: staged.status,
                    content_hash: staged.content_hash,
                    detected_at: chrono::Utc::now().timestamp_millis(),
                    has_conflict: staged.has_conflict,
                },
            )
        })
    }

    pub fn diff_doc_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        let (path, old_content, new_content) =
            self.workdir_diff_inputs_for_target_in_local_repo(repo_name, target)?;
        Ok(diff::unified_diff(&old_content, &new_content, &path))
    }
}
