use super::{
    GitMirrorPushOptions, GitMirrorPushReport, collect_mapping_blockers, push_mirror,
    resolve_push_target, resolved_push_target, sanitize_remote_url, validate_push_name,
};
use crate::git_bridge::{
    GIT_MIRROR_COMMITS_TABLE, GitMirrorCommitState, GitMirrorPushError, GitMirrorRecord,
    GitMirrorRunOptions, get_record, run_pending_mirror,
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
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _repo_id) = crate::test_support::init_cataloged_repo(&ledger, &projection_base)
        .expect("init cataloged repo");
    let repo_root = repo
        .local_repo_workspace_root(repo.local_repo_name())
        .expect("workspace root");
    init_git_repo(&repo_root);
    (dir, repo, repo_root)
}

fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
    let abs = repo
        .local_repo_workspace_path(repo.local_repo_name(), path)
        .expect("workspace path");
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn commit_deve_file(repo: &RepoManager, path: &str, content: &str) -> CommitInfo {
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
    repo.stage_pending(path).expect("stage");
    repo.apply_external_changes().expect("apply external");
    repo.commit_source_control_changes("initial")
        .expect("commit")
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
        (":refs/heads/main", "branch"),
        ("HEAD:refs/heads/other", "branch"),
        ("feature..other", "branch"),
        ("topic.lock", "branch"),
        ("https://user:secret@example.invalid/repo.git", "remote"),
        ("../private/repo.git", "remote"),
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
fn unresolved_push_target_becomes_blocker_instead_of_panic() {
    let mut missing_both = Default::default();
    assert_eq!(resolved_push_target(&mut missing_both), None);
    assert_eq!(missing_both.blockers.len(), 2);
    assert!(
        missing_both
            .blockers
            .iter()
            .any(|blocker| blocker.reason.contains("remote was not resolved")),
        "{:?}",
        missing_both.blockers
    );
    assert!(
        missing_both
            .blockers
            .iter()
            .any(|blocker| blocker.reason.contains("branch was not resolved")),
        "{:?}",
        missing_both.blockers
    );

    let mut ready = super::GitMirrorPushReport {
        remote: Some("origin".into()),
        branch: Some("main".into()),
        ..Default::default()
    };
    assert_eq!(
        resolved_push_target(&mut ready),
        Some(("origin".into(), "main".into()))
    );
    assert!(ready.blockers.is_empty());
}

#[test]
fn git_push_report_redacts_remote_credentials_and_command_detail() {
    assert_eq!(
        sanitize_remote_url("https://user:secret@example.invalid/repo.git?token=hidden#frag"),
        Some("https://example.invalid/repo.git".into())
    );
    assert_eq!(
        sanitize_remote_url("ssh://git@example.invalid/repo.git"),
        Some("ssh://example.invalid/repo.git".into())
    );
    assert_eq!(
        sanitize_remote_url("git@example.invalid:repo.git"),
        Some("example.invalid:repo.git".into())
    );
    assert_eq!(
        sanitize_remote_url("user:secret@example.invalid:repo.git"),
        Some("example.invalid:repo.git".into())
    );
    assert_eq!(
        sanitize_remote_url("https://example.invalid:443/repo.git"),
        Some("https://example.invalid:443/repo.git".into())
    );
    assert_eq!(
        sanitize_remote_url("ssh://git@[::1]:22/repo.git"),
        Some("ssh://[::1]:22/repo.git".into())
    );
    for unsafe_remote in [
        "/home/private/repo.git",
        r"C:\Users\private\repo.git",
        "file:///home/private/repo.git",
        "relative/repo.git",
        "https://user:secret/repo.git",
        "https://example.invalid:token/repo.git",
        "https://[not-ipv6]:22/repo.git",
        "https://example.invalid/repo.git\nsecret",
    ] {
        assert_eq!(
            sanitize_remote_url(unsafe_remote),
            None,
            "{unsafe_remote:?}"
        );
    }

    let source = include_str!("../push.rs");
    assert!(!source.contains("blocker(\"git_command\", reason)"));
    assert!(source.contains("Git push command failed; inspect credential, network"));

    let dir = tempfile::tempdir().expect("tempdir");
    init_git_repo(dir.path());
    let mut report = super::GitMirrorPushReport::default();
    resolve_push_target(
        dir.path(),
        &GitMirrorPushOptions {
            remote: Some("https://user:secret@example.invalid/repo.git".into()),
            branch: Some("main".into()),
        },
        &mut report,
    );
    assert_eq!(report.remote, None);
    assert_eq!(report.remote_url, None);
    assert!(
        !serde_json::to_string(&report)
            .expect("report")
            .contains("secret")
    );
}

#[test]
fn git_push_report_rejects_corrupt_durable_object_id_without_projection() {
    let secret = "secret\nC:\\private\\repository";
    let records = vec![GitMirrorRecord {
        deve_commit_id: "deve-commit".into(),
        repo_id: uuid::Uuid::new_v4(),
        ledger_seq: 1,
        state: GitMirrorCommitState::Committed,
        git_commit_id: Some(secret.into()),
        last_error: None,
        failure_stage: None,
        failure_subject: None,
        failure_command: None,
        failure_exit_status: None,
        queued_at_ms: 1,
        updated_at_ms: 1,
        attempts: 1,
    }];
    let mut report = GitMirrorPushReport {
        head: Some("0123456789abcdef0123456789abcdef01234567".into()),
        ..Default::default()
    };

    collect_mapping_blockers(&records, &mut report);

    assert_eq!(report.blockers.len(), 1, "{report:?}");
    assert_eq!(report.blockers[0].location, "git_history_mapping");
    assert!(
        report.blockers[0]
            .reason
            .contains("invalid durable Git object identity")
    );
    assert!(
        !serde_json::to_string(&report)
            .expect("report")
            .contains(secret)
    );
}

#[test]
fn push_mirror_pushes_exported_head_to_remote() {
    let (dir, repo, repo_root) = new_repo();
    let commit = commit_deve_file(&repo, "note.md", "hello\n");
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
    commit_deve_file(&repo, "note.md", "hello\n");
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
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _repo_id) = crate::test_support::init_cataloged_repo(&ledger, &projection_base)
        .expect("init cataloged repo");
    let repo_root = repo
        .local_repo_workspace_root(repo.local_repo_name())
        .expect("workspace root");
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
