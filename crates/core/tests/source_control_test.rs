#[cfg(test)]
mod tests {
    //! Source Control 集成测试：覆盖 pending、staging、commit 的完整生命周期。

    use deve_core::ledger::RepoManager;
    use deve_core::source_control::pending_fs::{self, PendingFsEntry};
    use deve_core::source_control::{ChangeEntry, ChangeStatus};
    use tempfile::{TempDir, tempdir};

    fn new_repo() -> (TempDir, RepoManager) {
        let dir = tempdir().expect("create tempdir");
        let repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
        (dir, repo)
    }

    #[test]
    fn test_pending_fs_crud() {
        let (_dir, repo) = new_repo();
        let entry = PendingFsEntry {
            path: "notes/a.md".into(),
            change_type: ChangeStatus::Modified,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        };
        let (all, left) = repo
            .run_on_local_repo(repo.local_repo_name(), |db| {
                pending_fs::upsert(db, &entry)?;
                let all = pending_fs::list_all(db)?;
                pending_fs::remove(db, &entry.path)?;
                let left = pending_fs::list_all(db)?;
                Ok((all, left))
            })
            .expect("pending fs crud");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].path, entry.path);
        assert!(left.is_empty());
    }

    #[test]
    fn test_staging_lifecycle() {
        let (_dir, repo) = new_repo();
        repo.stage_file("notes/a.md").expect("stage a");
        repo.stage_file("notes/b.md").expect("stage b");
        let staged = repo.list_staged().expect("list staged");
        assert!(
            staged
                .iter()
                .any(|e| e.path == "notes/a.md" && e.status == ChangeStatus::Modified)
        );
        assert!(staged.iter().any(|e| e.path == "notes/b.md"));
        repo.unstage_file("notes/b.md").expect("unstage b");
        let staged2 = repo.list_staged().expect("list staged after unstage");
        assert_eq!(staged2.len(), 1);
        assert_eq!(staged2[0].path, "notes/a.md");
    }

    #[test]
    fn test_commit_flow() {
        let (_dir, repo) = new_repo();
        repo.stage_file("notes/a.md").expect("stage a");
        repo.stage_file("notes/b.md").expect("stage b");
        let c = repo.commit_staged("test commit").expect("commit staged");
        assert!(!c.id.is_empty());
        assert_eq!(c.message, "test commit");
        assert_eq!(c.doc_count, 2);
        assert!(c.parent_id.is_none());
        assert_eq!(c.ledger_seq, 0);
        assert!(repo.list_staged().expect("staged after commit").is_empty());
    }

    #[test]
    fn test_commit_history_chain() {
        let (_dir, repo) = new_repo();
        repo.stage_file("notes/a.md").expect("stage a");
        let c1 = repo.commit_staged("c1").expect("commit 1");
        repo.stage_file("notes/b.md").expect("stage b");
        let c2 = repo.commit_staged("c2").expect("commit 2");
        let commits = repo.list_commits(10).expect("list commits");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].id, c2.id);
        assert_eq!(commits[1].id, c1.id);
        assert_eq!(commits[0].parent_id.as_deref(), Some(c1.id.as_str()));
        assert!(commits[1].parent_id.is_none());
    }

    #[test]
    fn test_pending_to_staged() {
        let (_dir, repo) = new_repo();
        let path = "notes/pending.md";
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: path.into(),
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("new"),
                    detected_at: 2,
                    has_conflict: false,
                },
            )
        })
        .expect("seed pending");
        let pending: Vec<ChangeEntry> = repo.list_pending_fs().expect("list pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, path);
        repo.stage_pending(path).expect("stage pending");
        assert!(
            repo.list_pending_fs()
                .expect("pending after stage")
                .is_empty()
        );
        let staged = repo.list_staged().expect("staged after pending->staged");
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].path, path);
        assert_eq!(staged[0].status, ChangeStatus::Added);
    }
}
