use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use crate::source_control::{pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

impl RepoManager {
    /// 将旧的 path-only Source Control 入口提升为 tracked target。
    ///
    /// Invariants:
    /// - 若当前 path 已被 node projection 跟踪，则必须补上 `doc_id`。
    /// - 若 path 不是当前 tracked projection，只保留规范化 path，不猜测旧 mapping。
    pub(super) fn tracked_target_for_path_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<ScPathTarget> {
        let path = to_forward_slash(path);
        let doc_id = match self.tracked_docid_or_legacy_error_in_local_repo(repo_name, &path)? {
            Some(doc_id) => Some(doc_id),
            None => self.run_on_local_repo(repo_name, |db| {
                if let Some(entry) = pending_fs::get(db, &path)?
                    && entry.doc_id.is_some()
                {
                    return Ok(entry.doc_id);
                }
                if let Some(entry) = staging::get_staged(db, &path)?
                    && entry.doc_id.is_some()
                {
                    return Ok(entry.doc_id);
                }
                Ok(None)
            })?,
        };
        Ok(ScPathTarget { doc_id, path })
    }
}

#[cfg(test)]
mod tests {
    use super::RepoManager;
    use crate::source_control::ChangeStatus;
    use crate::source_control::pending_fs::{self, PendingFsEntry};

    #[test]
    fn path_wrapper_preserves_doc_identity_from_exact_pending_entry() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(dir.path().join("vault"));
        let doc_id = repo.apply_file_structure_in_local_repo(
            repo.local_repo_name(),
            "notes/a.md",
            None,
            "test",
        )?;
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "docs/a.md".into(),
                    renamed_from: Some("notes/a.md".into()),
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello"),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })?;

        let target =
            repo.tracked_target_for_path_in_local_repo(repo.local_repo_name(), "docs/a.md")?;
        assert_eq!(target.doc_id, Some(doc_id));
        Ok(())
    }
}
