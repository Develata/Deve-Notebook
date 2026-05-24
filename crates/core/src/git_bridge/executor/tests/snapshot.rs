use super::*;

#[test]
fn export_mirror_bootstraps_latest_projection_snapshot() {
    let (_dir, repo, repo_root) = new_repo_without_git();
    let first = commit_deve_file(&repo, "note.md", "hello\n");
    let second = commit_deve_modification(&repo, "note.md", "hello world\n");
    init_git_repo(&repo_root);

    let report = export_for_default_repo(&repo, &repo_root);

    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 1);
    let first_record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &first.id)?))
        .expect("get first");
    assert!(first_record.is_none());
    let second_record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &second.id)?))
        .expect("get second")
        .expect("second record");
    assert_eq!(second_record.state, GitMirrorCommitState::Committed);
    assert!(second_record.git_commit_id.is_some());
    assert_eq!(git(&repo_root, &["show", "HEAD:note.md"]), "hello world\n");
    assert_eq!(
        git(&repo_root, &["rev-list", "--count", "HEAD"]).trim(),
        "1"
    );
    assert_eq!(git(&repo_root, &["status", "--porcelain"]), "");
    let body = git(&repo_root, &["log", "-1", "--pretty=%B"]);
    assert!(body.contains(&format!("Deve-Commit-Id: {}", second.id)));
    assert!(body.contains(&format!("Deve-Ledger-Seq: {}", second.ledger_seq)));
}

#[test]
fn export_mirror_rejects_snapshot_bootstrap_on_existing_git_history() {
    let (_dir, repo, repo_root) = new_repo_without_git();
    let commit = commit_deve_file(&repo, "note.md", "hello\n");
    init_git_repo(&repo_root);
    git(&repo_root, &["add", ".gitignore"]);
    git(
        &repo_root,
        &["commit", "--no-gpg-sign", "-m", "manual baseline"],
    );

    let report = export_for_default_repo(&repo, &repo_root);

    assert_eq!(report.committed, 0);
    assert_eq!(report.out_of_sync, 1);
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &commit.id)?))
        .expect("get")
        .expect("record");
    assert_eq!(record.state, GitMirrorCommitState::OutOfSync);
    assert_eq!(
        record.failure_stage,
        Some(GitMirrorFailureStage::GitHistoryMapping)
    );
    assert!(
        record
            .last_error
            .as_deref()
            .is_some_and(|err| err.contains("requires empty Git history"))
    );
}

#[test]
fn export_mirror_rejects_snapshot_bootstrap_when_mirror_is_not_ready() {
    let (_dir, repo, repo_root) = new_repo_without_git();
    let commit = commit_deve_file(&repo, "note.md", "hello\n");
    init_git_repo_without_notegit_ignore(&repo_root);

    let report = export_for_default_repo(&repo, &repo_root);

    assert_eq!(report.committed, 0);
    assert_eq!(report.out_of_sync, 1);
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &commit.id)?))
        .expect("get")
        .expect("record");
    assert_eq!(
        record.failure_stage,
        Some(GitMirrorFailureStage::MirrorNotReady)
    );
    assert!(
        record
            .last_error
            .as_deref()
            .is_some_and(|err| err.contains("does not ignore .notegit"))
    );
}

#[cfg(unix)]
#[test]
fn export_mirror_propagates_snapshot_status_inspect_without_queueing() {
    use std::os::unix::fs::symlink;

    let (dir, repo, repo_root) = new_repo_without_git();
    let commit = commit_deve_file(&repo, "note.md", "hello\n");
    std::fs::create_dir_all(&repo_root).expect("repo root");
    let outside = dir.path().join("outside.gitignore");
    std::fs::write(&outside, ".notegit/\n").expect("outside gitignore");
    symlink(&outside, repo_root.join(".gitignore")).expect("symlink gitignore");
    let repo_id = repo
        .get_repo_info()
        .expect("repo info")
        .expect("present")
        .uuid;

    let err = repo
        .run_on_local_repo(repo.local_repo_name(), |db| -> anyhow::Result<_> {
            Ok(export_mirror(
                db,
                &repo_root,
                repo_id,
                GitMirrorRunOptions::default(),
            )?)
        })
        .expect_err("status inspect should propagate");

    assert!(
        matches!(
            err.downcast_ref::<crate::git_bridge::GitMirrorRunError>(),
            Some(crate::git_bridge::GitMirrorRunError::StatusInspect { source })
                if source.to_string().contains("refusing to follow symlinked .gitignore")
        ),
        "{err:?}"
    );
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &commit.id)?))
        .expect("get");
    assert!(record.is_none(), "{record:?}");
}

#[test]
fn export_mirror_propagates_snapshot_projection_storage_error_without_marking_record() {
    let (_dir, repo, repo_root) = new_repo_without_git();
    let commit = commit_deve_file(&repo, "note.md", "hello\n");
    init_git_repo(&repo_root);
    repo.run_on_local_repo(repo.local_repo_name(), |db| -> anyhow::Result<()> {
        let write = db.begin_write()?;
        write.delete_table(crate::ledger::schema::LEDGER_OPS)?;
        write.commit()?;
        Ok(())
    })
    .expect("delete ledger ops table");
    let repo_id = repo
        .get_repo_info()
        .expect("repo info")
        .expect("present")
        .uuid;

    let err = repo
        .run_on_local_repo(repo.local_repo_name(), |db| -> anyhow::Result<_> {
            Ok(export_mirror(
                db,
                &repo_root,
                repo_id,
                GitMirrorRunOptions::default(),
            )?)
        })
        .expect_err("snapshot projection storage failure should propagate");

    assert!(
        matches!(
            err.downcast_ref::<crate::git_bridge::GitMirrorRunError>(),
            Some(crate::git_bridge::GitMirrorRunError::CommitDiffStorage { message })
                if message.contains("failed to read ledger ops")
        ),
        "{err:?}"
    );
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &commit.id)?))
        .expect("get")
        .expect("record remains queued");
    assert_eq!(record.state, GitMirrorCommitState::Queued);
    assert_eq!(record.attempts, 0);
    assert!(record.last_error.is_none(), "{record:?}");
}
