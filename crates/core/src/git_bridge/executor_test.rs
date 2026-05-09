use super::{GitMirrorRunOptions, export_mirror, run_pending_mirror};
use crate::git_bridge::{
    GitMirrorCommitState, GitMirrorFailureStage, get_record, queue_deve_commit,
};
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

fn init_git_repo_without_notegit_ignore(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git(path, &["init"]);
    git(path, &["config", "user.email", "deve@example.invalid"]);
    git(path, &["config", "user.name", "Deve Test"]);
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

fn new_repo_without_git() -> (TempDir, RepoManager, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    let repo_root = dir.path().join("vault").join("default");
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
    seed_pending_with_doc(repo, path, None, status, content);
}

fn seed_pending_with_doc(
    repo: &RepoManager,
    path: &str,
    doc_id: Option<crate::models::DocId>,
    status: ChangeStatus,
    content: &str,
) {
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
    .expect("seed pending");
}

fn commit_deve_file(dir: &TempDir, repo: &RepoManager, path: &str, content: &str) -> CommitInfo {
    write_workspace_file(dir, path, content);
    seed_pending(repo, path, ChangeStatus::Added, content);
    repo.stage_pending(path).expect("stage");
    repo.commit_staged("initial").expect("commit")
}

fn commit_deve_modification(
    dir: &TempDir,
    repo: &RepoManager,
    path: &str,
    content: &str,
) -> CommitInfo {
    let doc_id = repo.get_docid(path).expect("lookup doc").expect("doc id");
    write_workspace_file(dir, path, content);
    seed_pending_with_doc(repo, path, Some(doc_id), ChangeStatus::Modified, content);
    repo.stage_pending(path).expect("stage");
    repo.commit_staged("modify").expect("commit")
}

fn run_for_default_repo(repo: &RepoManager, repo_root: &Path) -> super::GitMirrorRunReport {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        run_pending_mirror(db, repo_root, GitMirrorRunOptions::default())
    })
    .expect("run mirror")
}

fn export_for_default_repo(repo: &RepoManager, repo_root: &Path) -> super::GitMirrorRunReport {
    let repo_id = repo
        .get_repo_info()
        .expect("repo info")
        .expect("present")
        .uuid;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        export_mirror(db, repo_root, repo_id, GitMirrorRunOptions::default())
    })
    .expect("export mirror")
}

#[path = "executor_snapshot_test.rs"]
mod snapshot;

#[path = "executor_preflight_test.rs"]
mod preflight;

#[test]
fn run_pending_mirror_commits_single_queued_record() {
    let (dir, repo, repo_root) = new_repo();
    let commit = commit_deve_file(&dir, &repo, "note.md", "hello\n");

    let report = run_for_default_repo(&repo, &repo_root);

    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 1);
    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &commit.id)?))
        .expect("get")
        .expect("record");
    assert_eq!(record.state, GitMirrorCommitState::Committed);
    assert!(record.git_commit_id.is_some());
    let body = git(&repo_root, &["log", "-1", "--pretty=%B"]);
    assert!(body.contains(&format!("Deve-Commit-Id: {}", commit.id)));
    assert!(body.contains(&format!("Deve-Ledger-Seq: {}", commit.ledger_seq)));
}

#[test]
fn run_pending_mirror_replays_multiple_queued_records() {
    let (dir, repo, repo_root) = new_repo();
    let first = commit_deve_file(&dir, &repo, "note.md", "hello\n");
    let second = commit_deve_modification(&dir, &repo, "note.md", "hello world\n");

    let report = run_for_default_repo(&repo, &repo_root);

    assert_eq!(report.attempted, 2);
    assert_eq!(report.committed, 2);
    let first_record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &first.id)?))
        .expect("get first")
        .expect("first record");
    let second_record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| Ok(get_record(db, &second.id)?))
        .expect("get second")
        .expect("second record");
    assert_eq!(first_record.state, GitMirrorCommitState::Committed);
    assert_eq!(second_record.state, GitMirrorCommitState::Committed);
    let first_git = first_record.git_commit_id.expect("first git commit");
    let first_spec = format!("{first_git}:note.md");
    assert_eq!(git(&repo_root, &["show", &first_spec]), "hello\n");
    assert_eq!(git(&repo_root, &["show", "HEAD:note.md"]), "hello world\n");
    assert_eq!(
        git(&repo_root, &["rev-list", "--count", "HEAD"]).trim(),
        "2"
    );
    assert_eq!(git(&repo_root, &["status", "--porcelain"]), "");
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
