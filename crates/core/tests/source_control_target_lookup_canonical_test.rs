use deve_core::ledger::RepoManager;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
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

#[test]
fn workdir_diff_target_rejects_doc_id_when_requested_path_is_not_in_change_set() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "hello");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed add");
    repo.stage_pending("notes/a.md").expect("stage add");
    repo.commit_staged("initial").expect("commit add");

    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");

    let target = ScPathTarget {
        path: "stale/a.md".into(),
        doc_id: Some(doc_id),
    };
    let err = repo
        .workdir_diff_inputs_for_target_in_local_repo(repo.local_repo_name(), &target)
        .expect_err("stale doc target must fail closed");

    assert!(
        err.to_string()
            .contains("Source control target not resolved")
    );
}

#[test]
fn workdir_diff_target_accepts_current_projection_path_without_change_entry() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "hello");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed add");
    repo.stage_pending("notes/a.md").expect("stage add");
    repo.commit_staged("initial").expect("commit add");

    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    write_workspace_file(&repo, "notes/a.md", "world");

    let (path, old_content, new_content) = repo
        .workdir_diff_inputs_for_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget {
                path: "notes/a.md".into(),
                doc_id: Some(doc_id),
            },
        )
        .expect("current projection target should resolve without pending row");

    assert_eq!(path, "notes/a.md");
    assert_eq!(old_content, "hello");
    assert_eq!(new_content, "world");
}

#[test]
fn workdir_diff_payload_preserves_doc_id_when_resolved_path_is_reused() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "A");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("A"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed a");
    repo.stage_pending("notes/a.md").expect("stage a");
    repo.commit_staged("commit a").expect("commit a");
    let doc_a = repo
        .get_docid("notes/a.md")
        .expect("lookup a")
        .expect("doc a");

    write_workspace_file(&repo, "notes/b.md", "B");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("B"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed b");
    repo.stage_pending("notes/b.md").expect("stage b");
    repo.commit_staged("commit b").expect("commit b");
    let doc_b = repo
        .get_docid("notes/b.md")
        .expect("lookup b")
        .expect("doc b");

    assert_ne!(doc_a, doc_b);
    write_workspace_file(&repo, "notes/a.md", "B changed");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_b),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("B changed"),
                detected_at: 3,
                has_conflict: false,
            },
        )
    })
    .expect("seed reused path modification");

    let (doc_id, path, old_content, new_content) = repo
        .workdir_diff_payload_for_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget {
                path: "notes/a.md".into(),
                doc_id: Some(doc_b),
            },
        )
        .expect("payload should preserve requested doc identity");

    assert_eq!(doc_id, Some(doc_b));
    assert_eq!(path, "notes/a.md");
    assert_eq!(old_content, "B");
    assert_eq!(new_content, "B changed");
}
