#[cfg(test)]
mod tests {
    //! Source Control 集成测试：覆盖 pending、staging、commit 的完整生命周期。

    use deve_core::ledger::RepoManager;
    use deve_core::source_control::pending_fs::{self, PendingFsEntry};
    use deve_core::source_control::{ChangeEntry, ChangeStatus};
    use deve_core::sync::scan;
    use deve_core::vfs::Vfs;
    use std::sync::Arc;
    use tempfile::{TempDir, tempdir};

    fn new_repo() -> (TempDir, RepoManager) {
        let dir = tempdir().expect("create tempdir");
        let mut repo =
            RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
        repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
            .expect("projection locator");
        (dir, repo)
    }

    fn seed_pending(repo: &RepoManager, path: &str, status: ChangeStatus, content: &str) {
        let doc_id = repo
            .get_tracked_docid_in_local_repo(repo.local_repo_name(), path)
            .expect("resolve tracked doc id for pending seed");
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: path.into(),
                    renamed_from: None,
                    doc_id,
                    change_type: status,
                    content_hash: pending_fs::content_hash(content),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })
        .expect("seed pending entry");
    }

    fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
        let abs = repo
            .local_repo_workspace_path("default", path)
            .expect("workspace path");
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).expect("create workspace parent");
        }
        std::fs::write(abs, content).expect("write workspace file");
    }

    fn workspace_path(repo: &RepoManager, path: &str) -> std::path::PathBuf {
        repo.local_repo_workspace_path("default", path)
            .expect("workspace path")
    }

    #[test]
    fn test_pending_fs_crud() {
        let (_dir, repo) = new_repo();
        let entry = PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
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
        seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "a");
        seed_pending(&repo, "notes/b.md", ChangeStatus::Added, "b");
        repo.stage_pending("notes/a.md").expect("stage a");
        repo.stage_pending("notes/b.md").expect("stage b");
        let staged = repo.list_staged().expect("list staged");
        assert!(
            staged
                .iter()
                .any(|e| e.path == "notes/a.md" && e.status == ChangeStatus::Modified)
        );
        assert!(
            staged
                .iter()
                .any(|e| e.path == "notes/b.md" && e.status == ChangeStatus::Added)
        );
        repo.unstage_file("notes/b.md").expect("unstage b");
        let staged2 = repo.list_staged().expect("list staged after unstage");
        assert_eq!(staged2.len(), 1);
        assert_eq!(staged2[0].path, "notes/a.md");
        let pending = repo.list_pending_fs().expect("pending after unstage");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, "notes/b.md");
        assert_eq!(pending[0].status, ChangeStatus::Added);
    }

    #[test]
    fn test_commit_flow() {
        let (_dir, repo) = new_repo();
        write_workspace_file(&repo, "notes/a.md", "hello");
        write_workspace_file(&repo, "notes/b.md", "world");
        seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
        seed_pending(&repo, "notes/b.md", ChangeStatus::Added, "world");
        repo.stage_pending("notes/a.md").expect("stage a");
        repo.stage_pending("notes/b.md").expect("stage b");
        repo.apply_external_changes()
            .expect("apply external changes");
        let c = repo
            .commit_staged_with_git_bridge("test commit", deve_core::config::GitBridgeMode::Mirror)
            .expect("commit staged");
        assert!(!c.id.is_empty());
        assert_eq!(c.message, "test commit");
        assert_eq!(c.doc_count, 2);
        assert!(c.parent_id.is_none());
        assert!(c.ledger_seq > 0);
        assert!(repo.list_staged().expect("staged after commit").is_empty());
    }

    #[test]
    fn test_commit_history_chain() {
        let (_dir, repo) = new_repo();
        write_workspace_file(&repo, "notes/a.md", "hello");
        seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
        repo.stage_pending("notes/a.md").expect("stage a");
        repo.apply_external_changes().expect("apply external c1");
        let c1 = repo
            .commit_staged_with_git_bridge("c1", deve_core::config::GitBridgeMode::Mirror)
            .expect("commit 1");
        write_workspace_file(&repo, "notes/b.md", "world");
        seed_pending(&repo, "notes/b.md", ChangeStatus::Added, "world");
        repo.stage_pending("notes/b.md").expect("stage b");
        repo.apply_external_changes().expect("apply external c2");
        let c2 = repo
            .commit_staged_with_git_bridge("c2", deve_core::config::GitBridgeMode::Mirror)
            .expect("commit 2");
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
                    renamed_from: None,
                    doc_id: None,
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

    #[test]
    fn test_discard_pending_modified_restores_ledger_projection() {
        let (_dir, repo) = new_repo();
        write_workspace_file(&repo, "notes/a.md", "hello");
        seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
        repo.stage_pending("notes/a.md").expect("stage a");
        repo.apply_external_changes()
            .expect("apply initial external change");
        repo.commit_staged_with_git_bridge("initial", deve_core::config::GitBridgeMode::Mirror)
            .expect("commit a");

        write_workspace_file(&repo, "notes/a.md", "world");
        seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "world");
        repo.discard_pending("notes/a.md")
            .expect("discard modified");

        let content =
            std::fs::read_to_string(workspace_path(&repo, "notes/a.md")).expect("read restored");
        assert_eq!(content, "hello");
        assert!(
            repo.list_pending_fs()
                .expect("pending after discard")
                .is_empty()
        );
    }

    #[test]
    fn test_diff_doc_path_reads_workspace_content() {
        let (_dir, repo) = new_repo();
        write_workspace_file(&repo, "notes/a.md", "hello");
        seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
        repo.stage_pending("notes/a.md").expect("stage a");
        repo.apply_external_changes()
            .expect("apply initial external change");
        repo.commit_staged_with_git_bridge("initial", deve_core::config::GitBridgeMode::Mirror)
            .expect("commit a");

        write_workspace_file(&repo, "notes/a.md", "world");
        let diff = repo.diff_doc_path("notes/a.md").expect("workdir diff");
        assert!(diff.contains("-hello"));
        assert!(diff.contains("+world"));
    }

    #[test]
    fn test_scan_marks_deleted_without_dropping_doc_mapping() {
        let (_dir, repo) = new_repo();
        write_workspace_file(&repo, "notes/a.md", "hello");
        seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
        repo.stage_pending("notes/a.md").expect("stage a");
        repo.apply_external_changes()
            .expect("apply initial external change");
        repo.commit_staged_with_git_bridge("initial", deve_core::config::GitBridgeMode::Mirror)
            .expect("commit a");
        std::fs::remove_file(workspace_path(&repo, "notes/a.md")).expect("remove workspace file");

        let repo_root = repo
            .local_repo_workspace_root("default")
            .expect("workspace root");
        let repo = Arc::new(repo);
        let vfs = Vfs::new(repo_root);
        scan::scan_projection_workspaces(&repo, &vfs).expect("scan workspace");

        assert!(
            repo.get_docid("notes/a.md")
                .expect("docid lookup")
                .is_some()
        );
        let pending = repo.list_pending_fs().expect("pending after scan");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, "notes/a.md");
        assert_eq!(pending[0].status, ChangeStatus::Deleted);
    }
}
