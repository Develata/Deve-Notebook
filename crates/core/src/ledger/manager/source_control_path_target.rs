use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};

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
        let doc_id = match self.get_tracked_docid_in_local_repo(repo_name, &path)? {
            Some(doc_id) => Some(doc_id),
            None => {
                tracked_doc_id_from_changes(&self.list_changes_in_local_repo(repo_name)?, &path)?
            }
        };
        Ok(ScPathTarget { doc_id, path })
    }
}

fn tracked_doc_id_from_changes(
    entries: &[ChangeEntry],
    path: &str,
) -> Result<Option<crate::models::DocId>> {
    let exact = entries
        .iter()
        .filter(|entry| normalized(&entry.path) == path)
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|entry| {
            entry.status != ChangeStatus::Deleted
                && entry
                    .renamed_from
                    .as_ref()
                    .is_some_and(|old_path| normalized(old_path) == path)
        })
        .collect::<Vec<_>>();
    if exact
        .iter()
        .chain(renamed.iter())
        .all(|entry| entry.doc_id.is_none())
    {
        return Ok(None);
    }
    let live_exact = exact
        .iter()
        .copied()
        .filter(|entry| entry.status != ChangeStatus::Deleted)
        .collect::<Vec<_>>();
    let deleted_exact = exact
        .iter()
        .any(|entry| entry.status == ChangeStatus::Deleted);
    if live_exact.len() > 1 || renamed.len() > 1 {
        return Err(anyhow!(
            "Ambiguous source control path target: {} matched multiple live tracked entries",
            path
        ));
    }
    if deleted_exact && renamed.len() == 1 {
        return Ok(renamed[0].doc_id);
    }
    if !deleted_exact && !live_exact.is_empty() && !renamed.is_empty() {
        return Err(anyhow!(
            "Ambiguous source control path target: {} matched reused path and rename successor",
            path
        ));
    }
    if let Some(entry) = live_exact.into_iter().next() {
        return Ok(entry.doc_id);
    }
    if let Some(entry) = renamed.into_iter().next() {
        return Ok(entry.doc_id);
    }
    Ok((exact.len() == 1).then_some(exact[0].doc_id).flatten())
}

fn normalized(path: &str) -> String {
    to_forward_slash(path)
}

#[cfg(test)]
mod tests {
    use super::RepoManager;
    use crate::source_control::ChangeStatus;
    use crate::source_control::pending_fs::{self, PendingFsEntry};
    use crate::source_control::staging;

    #[test]
    fn path_wrapper_preserves_doc_identity_from_exact_pending_entry() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(dir.path().join("vault"));
        let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
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

    #[test]
    fn path_wrapper_preserves_doc_identity_from_renamed_from_pending_entry() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(dir.path().join("vault"));
        let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
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
            repo.tracked_target_for_path_in_local_repo(repo.local_repo_name(), "notes/a.md")?;
        assert_eq!(target.path, "notes/a.md");
        assert_eq!(target.doc_id, Some(doc_id));
        Ok(())
    }

    #[test]
    fn path_wrapper_fails_closed_when_old_path_is_reused() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(dir.path().join("vault"));
        let doc_id = crate::models::DocId::new();
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
            )?;
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("new"),
                    detected_at: 2,
                    has_conflict: false,
                },
            )
        })?;

        let err = repo
            .tracked_target_for_path_in_local_repo(repo.local_repo_name(), "notes/a.md")
            .expect_err("reused old path must fail closed");
        assert!(
            err.to_string()
                .contains("Ambiguous source control path target: notes/a.md")
        );
        Ok(())
    }

    #[test]
    fn stage_wrapper_stages_renamed_entry_from_old_path() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(dir.path().join("vault"));
        let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
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

        repo.stage_pending_in_local_repo(repo.local_repo_name(), "notes/a.md")?;
        let staged = repo
            .run_on_local_repo(repo.local_repo_name(), |db| -> anyhow::Result<_> {
                staging::get_staged(db, "docs/a.md")
            })?
            .expect("renamed entry staged");
        assert_eq!(staged.doc_id, Some(doc_id));
        assert_eq!(staged.renamed_from.as_deref(), Some("notes/a.md"));
        Ok(())
    }
}
