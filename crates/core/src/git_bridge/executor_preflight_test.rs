use super::*;

#[test]
fn run_pending_mirror_marks_noop_as_out_of_sync() {
    let (dir, repo, repo_root) = new_repo();
    let commit = commit_deve_file(&dir, &repo, "note.md", "hello\n");
    git(&repo_root, &["add", "-A"]);
    git(
        &repo_root,
        &["commit", "--no-gpg-sign", "-m", "manual baseline"],
    );

    let report = run_for_default_repo(&repo, &repo_root);

    assert_eq!(report.out_of_sync, 1);
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| get_record(db, &commit.id))
        .expect("get")
        .expect("record");
    assert_eq!(record.state, GitMirrorCommitState::OutOfSync);
    assert_eq!(
        record.failure_stage,
        Some(GitMirrorFailureStage::MirrorExecutor)
    );
    assert!(
        record
            .last_error
            .as_deref()
            .is_some_and(|err| err.contains("no staged changes"))
    );
}

#[test]
fn run_pending_mirror_rejects_pending_source_control_changes() {
    let (dir, repo, repo_root) = new_repo();
    let commit = commit_deve_file(&dir, &repo, "note.md", "hello\n");
    write_workspace_file(&dir, "draft.md", "draft\n");
    seed_pending(&repo, "draft.md", ChangeStatus::Added, "draft\n");

    let report = run_for_default_repo(&repo, &repo_root);

    assert_eq!(report.committed, 0);
    assert_eq!(report.out_of_sync, 1);
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| get_record(db, &commit.id))
        .expect("get")
        .expect("record");
    assert_eq!(
        record.failure_stage,
        Some(GitMirrorFailureStage::DeveSourceControl)
    );
    assert!(
        record
            .last_error
            .as_deref()
            .is_some_and(|err| err.contains("pending source-control change"))
    );
}

#[test]
fn run_pending_mirror_rejects_git_paths_outside_deve_commit() {
    let (dir, repo, repo_root) = new_repo();
    let commit = commit_deve_file(&dir, &repo, "note.md", "hello\n");
    write_workspace_file(&dir, "outside.md", "outside\n");

    let report = run_for_default_repo(&repo, &repo_root);

    assert_eq!(report.committed, 0);
    assert_eq!(report.out_of_sync, 1);
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| get_record(db, &commit.id))
        .expect("get")
        .expect("record");
    assert_eq!(
        record.failure_stage,
        Some(GitMirrorFailureStage::ProjectionScope)
    );
    assert!(
        record
            .last_error
            .as_deref()
            .is_some_and(|err| err.contains("outside queued Deve commit"))
    );
}

#[test]
fn run_pending_mirror_rejects_tracked_notegit_paths() {
    let (dir, repo, repo_root) = new_repo();
    std::fs::create_dir_all(repo_root.join(".notegit")).expect("notegit dir");
    std::fs::write(repo_root.join(".notegit").join("state"), "secret").expect("notegit state");
    git(&repo_root, &["add", ".gitignore"]);
    git(&repo_root, &["add", "-f", ".notegit/state"]);
    git(
        &repo_root,
        &["commit", "--no-gpg-sign", "-m", "bad baseline"],
    );
    let commit = commit_deve_file(&dir, &repo, "note.md", "hello\n");

    let report = run_for_default_repo(&repo, &repo_root);

    assert_eq!(report.committed, 0);
    assert_eq!(report.out_of_sync, 1);
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| get_record(db, &commit.id))
        .expect("get")
        .expect("record");
    assert_eq!(
        record.failure_stage,
        Some(GitMirrorFailureStage::NotegitProtection)
    );
    assert!(
        record
            .last_error
            .as_deref()
            .is_some_and(|err| err.contains("already tracked by Git"))
    );
}
