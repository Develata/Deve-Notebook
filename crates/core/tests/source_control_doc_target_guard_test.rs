use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, SourceControlApi, staging};
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
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(abs, content).expect("write workspace");
}

fn commit_doc(_dir: &TempDir, repo: &RepoManager, path: &str, content: &str) -> DocId {
    write_workspace_file(repo, path, content);
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");
    repo.stage_pending(path).expect("stage doc");
    repo.commit_staged_with_git_bridge("commit doc", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit doc");
    repo.get_docid(path).expect("lookup").expect("existing")
}

#[test]
fn stage_target_with_doc_id_does_not_fall_back_to_other_doc_path() {
    let (dir, repo) = new_repo();
    let doc_a = commit_doc(&dir, &repo, "notes/a.md", "alpha");
    let doc_b = commit_doc(&dir, &repo, "notes/b.md", "beta");
    assert_ne!(doc_a, doc_b);

    write_workspace_file(&repo, "notes/b.md", "beta changed");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: None,
                doc_id: Some(doc_b),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("beta changed"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending b");

    let err = <RepoManager as SourceControlApi>::stage_pending_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget {
            path: "notes/b.md".into(),
            doc_id: Some(doc_a),
        },
    )
    .expect_err("mismatched doc target must fail");
    assert!(err.to_string().contains("Path is not in pending_fs_ops"));
}

#[test]
fn unstage_target_with_doc_id_does_not_fall_back_to_other_doc_path() {
    let (dir, repo) = new_repo();
    let doc_a = commit_doc(&dir, &repo, "notes/a.md", "alpha");
    let doc_b = commit_doc(&dir, &repo, "notes/b.md", "beta");
    assert_ne!(doc_a, doc_b);

    write_workspace_file(&repo, "notes/b.md", "beta changed");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: None,
                doc_id: Some(doc_b),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("beta changed"),
                detected_at: 2,
                has_conflict: false,
            },
        )?;
        let entry = pending_fs::take_for_target(
            db,
            &ScPathTarget {
                path: "notes/b.md".into(),
                doc_id: Some(doc_b),
            },
        )?
        .expect("pending b");
        staging::stage_pending_entry(db, &entry)
    })
    .expect("seed staged b");

    let err = <RepoManager as SourceControlApi>::unstage_file_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget {
            path: "notes/b.md".into(),
            doc_id: Some(doc_a),
        },
    )
    .expect_err("mismatched staged doc target must fail");
    assert!(err.to_string().contains("Path is not staged"));
}

#[test]
fn stage_path_only_target_fails_closed_for_tracked_entry() {
    let (dir, repo) = new_repo();
    let doc_id = commit_doc(&dir, &repo, "notes/b.md", "beta");

    write_workspace_file(&repo, "notes/b.md", "beta changed");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("beta changed"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed tracked pending");

    let err = <RepoManager as SourceControlApi>::stage_pending_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget::from_path("notes/b.md"),
    )
    .expect_err("tracked path-only stage must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous pending_fs target: notes/b.md")
    );
}

#[test]
fn unstage_path_only_target_fails_closed_for_tracked_entry() {
    let (dir, repo) = new_repo();
    let doc_id = commit_doc(&dir, &repo, "notes/b.md", "beta");

    write_workspace_file(&repo, "notes/b.md", "beta changed");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("beta changed"),
                detected_at: 2,
                has_conflict: false,
            },
        )?;
        let entry = pending_fs::take_for_target(
            db,
            &ScPathTarget {
                path: "notes/b.md".into(),
                doc_id: Some(doc_id),
            },
        )?
        .expect("pending b");
        staging::stage_pending_entry(db, &entry)
    })
    .expect("seed tracked staged");

    let err = <RepoManager as SourceControlApi>::unstage_file_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget::from_path("notes/b.md"),
    )
    .expect_err("tracked path-only unstage must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous staged target: notes/b.md")
    );
}
