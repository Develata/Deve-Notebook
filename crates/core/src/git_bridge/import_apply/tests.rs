use super::apply_import;
use crate::git_bridge::plan_import;
use crate::ledger::RepoManager;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
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

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git(path, &["init"]);
    git(path, &["config", "user.email", "deve@example.invalid"]);
    git(path, &["config", "user.name", "Deve Test"]);
    crate::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

fn new_repo() -> (TempDir, RepoManager, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos(dir.path().join("vault"));
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

fn commit_deve_file(dir: &TempDir, repo: &RepoManager, path: &str, content: &str) {
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
    repo.commit_staged("initial").expect("commit");
}

fn commit_git_baseline(repo_root: &Path) {
    git(repo_root, &["add", "."]);
    git(repo_root, &["commit", "--no-gpg-sign", "-m", "baseline"]);
}

#[test]
fn apply_import_writes_modified_and_added_pending_entries() {
    let (dir, repo, repo_root) = new_repo();
    commit_deve_file(&dir, &repo, "note.md", "hello\n");
    commit_git_baseline(&repo_root);
    write_workspace_file(&dir, "note.md", "hello import\n");
    write_workspace_file(&dir, "new.md", "new file\n");

    let report = apply_import(&repo, repo.local_repo_name(), &repo_root).expect("apply import");

    assert_eq!(report.applied, 2);
    assert_eq!(report.skipped, 0);
    assert!(report.blockers.is_empty(), "{:?}", report.blockers);
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), pending_fs::list_all)
        .expect("pending");
    assert!(
        pending.iter().any(|entry| {
            entry.path == "note.md"
                && entry.change_type == ChangeStatus::Modified
                && entry.doc_id.is_some()
        }),
        "{pending:?}"
    );
    assert!(
        pending.iter().any(|entry| {
            entry.path == "new.md"
                && entry.change_type == ChangeStatus::Added
                && entry.doc_id.is_none()
        }),
        "{pending:?}"
    );
}

#[test]
fn plan_import_dry_run_does_not_write_pending_entries() {
    let (dir, repo, repo_root) = new_repo();
    commit_deve_file(&dir, &repo, "note.md", "hello\n");
    commit_git_baseline(&repo_root);
    write_workspace_file(&dir, "note.md", "hello import\n");
    write_workspace_file(&dir, "new.md", "new file\n");

    let plan = plan_import(&repo_root).expect("plan import");

    assert_eq!(plan.entries.len(), 2);
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), pending_fs::list_all)
        .expect("pending");
    assert!(pending.is_empty(), "{pending:?}");
}

#[test]
fn apply_import_writes_renamed_pending_entry() {
    let (dir, repo, repo_root) = new_repo();
    commit_deve_file(&dir, &repo, "note.md", "hello\n");
    commit_git_baseline(&repo_root);
    git(&repo_root, &["mv", "note.md", "moved.md"]);

    let report = apply_import(&repo, repo.local_repo_name(), &repo_root).expect("apply import");

    assert_eq!(report.applied, 1);
    assert!(report.blockers.is_empty(), "{:?}", report.blockers);
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), pending_fs::list_all)
        .expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "moved.md");
    assert_eq!(pending[0].renamed_from.as_deref(), Some("note.md"));
    assert_eq!(pending[0].change_type, ChangeStatus::Renamed);
    assert!(pending[0].doc_id.is_some());
}

#[test]
fn apply_import_reports_blocker_without_writing_when_source_control_staged_exists() {
    let (dir, repo, repo_root) = new_repo();
    commit_deve_file(&dir, &repo, "note.md", "hello\n");
    commit_git_baseline(&repo_root);
    write_workspace_file(&dir, "note.md", "hello import\n");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "other.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("other"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");
    repo.stage_pending("other.md").expect("seed staged");

    let report = apply_import(&repo, repo.local_repo_name(), &repo_root).expect("apply import");

    assert_eq!(report.applied, 0);
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| blocker.reason.contains("source-control staged"))
    );
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), pending_fs::list_all)
        .expect("pending");
    assert!(pending.is_empty(), "{pending:?}");
}

#[test]
fn apply_import_existing_pending_blocker_prevents_partial_writes() {
    let (dir, repo, repo_root) = new_repo();
    commit_deve_file(&dir, &repo, "note.md", "hello\n");
    commit_git_baseline(&repo_root);
    write_workspace_file(&dir, "note.md", "hello import\n");
    write_workspace_file(&dir, "new.md", "new file\n");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "note.md".into(),
                renamed_from: None,
                doc_id: repo.get_docid("note.md").expect("lookup doc"),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("different pending"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed existing pending");

    let report = apply_import(&repo, repo.local_repo_name(), &repo_root).expect("apply import");

    assert_eq!(report.applied, 0);
    assert_eq!(report.skipped, 0);
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| blocker.reason.contains("existing pending entry")),
        "{:?}",
        report.blockers
    );
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), pending_fs::list_all)
        .expect("pending");
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0].path, "note.md");
    assert!(!pending.iter().any(|entry| entry.path == "new.md"));
}
