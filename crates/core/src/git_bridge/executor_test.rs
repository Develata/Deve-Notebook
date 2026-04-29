use super::{GitMirrorRunOptions, run_pending_mirror};
use crate::git_bridge::{GitMirrorCommitState, get_record, queue_deve_commit};
use crate::ledger::RepoManager;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::source_control::{ChangeStatus, CommitInfo, commits};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn commit(id: &str, ledger_seq: u64) -> CommitInfo {
    CommitInfo {
        id: id.to_string(),
        parent_id: None,
        message: "commit".to_string(),
        timestamp: 1,
        doc_count: 1,
        ledger_seq,
    }
}

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git(path, &["init"]);
    git(path, &["config", "user.email", "deve@example.invalid"]);
    git(path, &["config", "user.name", "Deve Test"]);
    crate::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

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

fn seed_pending(repo: &RepoManager, path: &str, status: ChangeStatus, content: &str) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id: None,
                change_type: status,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");
}

fn commit_deve_file(dir: &TempDir, repo: &RepoManager, path: &str, content: &str) -> CommitInfo {
    write_workspace_file(dir, path, content);
    seed_pending(repo, path, ChangeStatus::Added, content);
    repo.stage_pending(path).expect("stage");
    repo.commit_staged("initial").expect("commit")
}

fn run_for_default_repo(repo: &RepoManager, repo_root: &Path) -> super::GitMirrorRunReport {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        run_pending_mirror(db, repo_root, GitMirrorRunOptions::default())
    })
    .expect("run mirror")
}

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
    assert!(
        record
            .last_error
            .as_deref()
            .is_some_and(|err| err.contains("already tracked by Git"))
    );
}

#[test]
fn run_pending_mirror_rejects_multiple_queued_records_without_fake_mapping() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo_root = dir.path().join("repo");
    init_git_repo(&repo_root);
    std::fs::write(repo_root.join("note.md"), "hello\n").expect("write note");
    let db = redb::Database::create(dir.path().join("mirror.redb")).expect("db");
    pending_fs::init_table(&db).expect("pending table");
    crate::source_control::staging::init_table(&db).expect("staged table");
    commits::init_table(&db).expect("commits table");
    let repo_id = uuid::Uuid::new_v4();
    queue_deve_commit(&db, repo_id, &commit("deve-1", 7)).expect("queue first");
    queue_deve_commit(&db, repo_id, &commit("deve-2", 8)).expect("queue second");

    let report =
        run_pending_mirror(&db, &repo_root, GitMirrorRunOptions::default()).expect("run mirror");

    assert_eq!(report.committed, 0);
    assert_eq!(report.out_of_sync, 2);
    let output = Command::new("git")
        .arg("-C")
        .arg(&repo_root)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .expect("run git rev-list");
    assert!(!output.status.success());
}
