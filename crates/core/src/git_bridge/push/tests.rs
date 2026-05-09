use super::{GitMirrorPushOptions, push_mirror, validate_push_name};
use crate::git_bridge::{
    GIT_MIRROR_COMMITS_TABLE, GitMirrorCommitState, GitMirrorPushError, GitMirrorRunOptions,
    get_record, run_pending_mirror,
};
use crate::ledger::RepoManager;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::source_control::{ChangeStatus, CommitInfo};
use redb::TableHandle;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn git_status(path: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("run git")
        .status
        .success()
}

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git(path, &["init"]);
    git(path, &["config", "user.email", "deve@example.invalid"]);
    git(path, &["config", "user.name", "Deve Test"]);
    crate::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

fn init_bare_remote(path: &Path) {
    std::fs::create_dir_all(path).expect("remote dir");
    git(path, &["init", "--bare"]);
}

fn new_repo() -> (TempDir, RepoManager, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let repo_root = dir.path().join("vault").join("default");
    init_git_repo(&repo_root);
    (dir, repo, repo_root)
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn commit_deve_file(dir: &TempDir, repo: &RepoManager, path: &str, content: &str) -> CommitInfo {
    write_workspace_file(dir, path, content);
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
    repo.stage_pending(path).expect("stage");
    repo.commit_staged("initial").expect("commit")
}

fn mirror_queued(repo: &RepoManager, repo_root: &Path) {
    let report = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            Ok(run_pending_mirror(
                db,
                repo_root,
                GitMirrorRunOptions::default(),
            )?)
        })
        .expect("run mirror");
    assert_eq!(report.out_of_sync, 0);
    assert!(report.committed > 0);
}

fn current_branch(repo_root: &Path) -> String {
    git(repo_root, &["branch", "--show-current"])
        .trim()
        .to_string()
}

#[test]
fn push_name_validation_rejects_option_like_or_whitespace_values() {
    for (value, label) in [
        ("--mirror", "remote"),
        ("origin --upload-pack=sh", "remote"),
        ("feature branch", "branch"),
        ("", "branch"),
    ] {
        let err = validate_push_name(value, label)
            .expect_err("invalid push target must be rejected")
            .to_string();
        assert!(err.contains("Git push mirror refuses invalid"), "{err:?}");
        assert!(err.contains(label), "{err:?}");
        assert!(err.contains(value), "{err:?}");
    }
}

#[test]
fn push_mirror_pushes_exported_head_to_remote() {
    let (dir, repo, repo_root) = new_repo();
    let commit = commit_deve_file(&dir, &repo, "note.md", "hello\n");
    mirror_queued(&repo, &repo_root);
    let remote = dir.path().join("remote.git");
    init_bare_remote(&remote);
    git(
        &repo_root,
        &["remote", "add", "origin", remote.to_str().expect("remote")],
    );
    let branch = current_branch(&repo_root);

    let report = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            Ok(push_mirror(
                db,
                &repo_root,
                GitMirrorPushOptions {
                    remote: Some("origin".into()),
                    branch: Some(branch.clone()),
                },
            )?)
        })
        .expect("push mirror");

    assert!(report.pushed, "{report:?}");
    assert!(report.blockers.is_empty(), "{:?}", report.blockers);
    assert_eq!(report.remote.as_deref(), Some("origin"));
    assert_eq!(report.branch.as_deref(), Some(branch.as_str()));
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &commit.id)?))
        .expect("record")
        .expect("present");
    assert_eq!(record.state, GitMirrorCommitState::Committed);
    let remote_head = git(&remote, &["rev-parse", &format!("refs/heads/{branch}")]);
    assert_eq!(
        remote_head.trim(),
        record.git_commit_id.as_deref().expect("git commit")
    );
}

#[test]
fn push_mirror_refuses_unexported_queue_without_touching_remote() {
    let (dir, repo, repo_root) = new_repo();
    commit_deve_file(&dir, &repo, "note.md", "hello\n");
    let remote = dir.path().join("remote.git");
    init_bare_remote(&remote);
    git(
        &repo_root,
        &["remote", "add", "origin", remote.to_str().expect("remote")],
    );

    let report = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            Ok(push_mirror(
                db,
                &repo_root,
                GitMirrorPushOptions {
                    remote: Some("origin".into()),
                    branch: Some("main".into()),
                },
            )?)
        })
        .expect("push mirror");

    assert!(!report.pushed);
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| blocker.reason.contains("unpublished mirror records")),
        "{:?}",
        report.blockers
    );
    assert!(!git_status(
        &remote,
        &["rev-parse", "--verify", "refs/heads/main"]
    ));
}

#[test]
fn push_mirror_refuses_git_head_without_deve_mapping() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let repo_root = dir.path().join("vault").join("default");
    init_git_repo(&repo_root);
    git(&repo_root, &["add", ".gitignore"]);
    git(
        &repo_root,
        &["commit", "--no-gpg-sign", "-m", "manual baseline"],
    );
    let remote = dir.path().join("remote.git");
    init_bare_remote(&remote);
    git(
        &repo_root,
        &["remote", "add", "origin", remote.to_str().expect("remote")],
    );
    let branch = current_branch(&repo_root);

    let report = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            Ok(push_mirror(
                db,
                &repo_root,
                GitMirrorPushOptions {
                    remote: Some("origin".into()),
                    branch: Some(branch),
                },
            )?)
        })
        .expect("push mirror");

    assert!(!report.pushed);
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| blocker.reason.contains("without Deve mirror mapping")),
        "{:?}",
        report.blockers
    );
}

#[test]
fn push_mirror_propagates_store_error_instead_of_blocker() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo_root = dir.path().join("repo");
    init_git_repo(&repo_root);
    let db = redb::Database::create(dir.path().join("mirror.redb")).expect("db");
    {
        let txn = db.begin_write().expect("write txn");
        {
            let _ = txn
                .open_table(redb::TableDefinition::<u64, u64>::new(
                    GIT_MIRROR_COMMITS_TABLE.name(),
                ))
                .expect("wrong table type");
        }
        txn.commit().expect("commit wrong table");
    }

    let err = push_mirror(
        &db,
        &repo_root,
        GitMirrorPushOptions {
            remote: Some("origin".into()),
            branch: Some("main".into()),
        },
    )
    .expect_err("store errors must not become user repair blockers");

    assert!(
        matches!(err, GitMirrorPushError::Store(_)),
        "unexpected error: {err:?}"
    );
}
