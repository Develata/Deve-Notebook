//! Shared Git import source-control test fixtures.

use crate::server::AppState;
use deve_core::git_bridge::{
    GitMirrorCommitState, GitMirrorRunOptions, apply_import, export_mirror, get_record,
};
use deve_core::models::DocId;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;

#[path = "source_control_scope_test_support.rs"]
mod scope_support;

pub(super) struct ImportedConflictFixture {
    pub(super) _dir: TempDir,
    pub(super) state: Arc<AppState>,
    pub(super) repo_id: uuid::Uuid,
    pub(super) repo_name: String,
    pub(super) repo_root: PathBuf,
    pub(super) doc_id: DocId,
    pub(super) before_commit_count: usize,
}

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    scope_support::build_state()
}

pub(super) fn write_workspace_file(dir: &TempDir, repo_name: &str, path: &str, content: &str) {
    scope_support::write_workspace_file(dir, repo_name, path, content);
}

pub(super) fn git(path: &Path, args: &[&str]) -> String {
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

pub(super) fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git(path, &["init"]);
    git(path, &["config", "user.email", "deve@example.invalid"]);
    git(path, &["config", "user.name", "Deve Test"]);
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

pub(super) fn create_imported_conflict_fixture() -> anyhow::Result<ImportedConflictFixture> {
    let (dir, state, repo_id, _test_id) = build_state()?;
    let repo_name = state.repo.local_repo_name().to_string();
    let repo_root = state.repo.local_repo_workspace_root(&repo_name)?;
    init_git_repo(&repo_root);
    seed_note_baseline(&dir, &state, &repo_name, "hello\n")?;
    git(&repo_root, &["add", "."]);
    git(&repo_root, &["commit", "--no-gpg-sign", "-m", "baseline"]);

    finish_imported_conflict_fixture(dir, state, repo_id, repo_name, repo_root)
}

pub(super) fn create_mapped_imported_conflict_fixture() -> anyhow::Result<ImportedConflictFixture> {
    let (dir, state, repo_id, _test_id) = build_state()?;
    let repo_name = state.repo.local_repo_name().to_string();
    let repo_root = state.repo.local_repo_workspace_root(&repo_name)?;
    init_git_repo(&repo_root);
    let baseline_commit = seed_note_baseline(&dir, &state, &repo_name, "hello\n")?;
    let baseline_report = state.repo.run_on_local_repo(&repo_name, |db| {
        export_mirror(db, &repo_root, repo_id, GitMirrorRunOptions::default())
    })?;
    assert_eq!(baseline_report.attempted, 1, "{baseline_report:?}");
    assert_eq!(baseline_report.committed, 1, "{baseline_report:?}");
    state.repo.run_on_local_repo(&repo_name, |db| {
        let record = get_record(db, &baseline_commit)?.expect("baseline mirror record");
        assert_eq!(record.state, GitMirrorCommitState::Committed);
        assert!(record.git_commit_id.is_some(), "{record:?}");
        Ok::<_, anyhow::Error>(())
    })?;
    assert!(git(&repo_root, &["status", "--porcelain"]).is_empty());

    finish_imported_conflict_fixture(dir, state, repo_id, repo_name, repo_root)
}

fn seed_note_baseline(
    dir: &TempDir,
    state: &Arc<AppState>,
    repo_name: &str,
    content: &str,
) -> anyhow::Result<String> {
    write_workspace_file(dir, repo_name, "note.md", content);
    state.repo.run_on_local_repo(repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "note.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;
    state.repo.stage_pending_in_local_repo(repo_name, "note.md")?;
    Ok(state
        .repo
        .commit_staged_in_local_repo(repo_name, "baseline")?
        .id)
}

fn finish_imported_conflict_fixture(
    dir: TempDir,
    state: Arc<AppState>,
    repo_id: uuid::Uuid,
    repo_name: String,
    repo_root: PathBuf,
) -> anyhow::Result<ImportedConflictFixture> {
    let doc_id = state
        .repo
        .get_tracked_docid_in_local_repo(&repo_name, "note.md")?
        .expect("doc id");
    state.repo.append_generated_op_in_local_repo(
        &repo_name,
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 6,
                    content: "ledger\n".into(),
                },
                2,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )?;
    write_workspace_file(&dir, &repo_name, "note.md", "git import\n");

    let report = apply_import(&state.repo, &repo_name, &repo_root)?;

    assert_eq!(report.applied, 1);
    let pending = state.repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert!(pending[0].has_conflict, "{pending:?}");
    let before_commit_count = state.repo.list_commits_in_local_repo(&repo_name, 10)?.len();

    Ok(ImportedConflictFixture {
        _dir: dir,
        state,
        repo_id,
        repo_name,
        repo_root,
        doc_id,
        before_commit_count,
    })
}
