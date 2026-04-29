//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Explicit Git mirror executor. It never acts as source-control authority.

use super::status::{GitMirrorState, inspect_repo_root};
use super::store::{
    GitMirrorCommitState, GitMirrorRecord, list_records, mark_committed, mark_out_of_sync,
};
use crate::source_control::{self, CommitInfo};
use crate::utils::{notegit, path::to_forward_slash};
use anyhow::Result;
use redb::Database;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GitMirrorRunOptions {
    pub retry_out_of_sync: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitMirrorRunReport {
    pub attempted: usize,
    pub committed: usize,
    pub out_of_sync: usize,
    pub skipped: usize,
    pub records: Vec<GitMirrorRecord>,
}

pub fn run_pending_mirror(
    db: &Database,
    repo_root: &Path,
    options: GitMirrorRunOptions,
) -> Result<GitMirrorRunReport> {
    let candidates = pending_candidates(db, options.retry_out_of_sync)?;
    if candidates.is_empty() {
        return Ok(GitMirrorRunReport::default());
    }

    let status = inspect_repo_root(repo_root)?;
    if status.state != GitMirrorState::Ready {
        let reason = status.reason.unwrap_or_else(|| {
            format!(
                "Git mirror is not ready: state={} git={}",
                status.state.as_str(),
                status.git_metadata_kind.as_str()
            )
        });
        return mark_all_out_of_sync(db, candidates, reason);
    }

    if candidates.len() != 1 {
        return mark_all_out_of_sync(
            db,
            candidates,
            "multiple queued Git mirror records require projection replay".to_string(),
        );
    }

    run_one_candidate(db, repo_root, &candidates[0])
}

fn pending_candidates(db: &Database, retry_out_of_sync: bool) -> Result<Vec<GitMirrorRecord>> {
    Ok(list_records(db)?
        .into_iter()
        .filter(|record| {
            record.state == GitMirrorCommitState::Queued
                || (retry_out_of_sync && record.state == GitMirrorCommitState::OutOfSync)
        })
        .collect())
}

fn run_one_candidate(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> Result<GitMirrorRunReport> {
    let mut report = GitMirrorRunReport {
        attempted: 1,
        ..GitMirrorRunReport::default()
    };
    match commit_worktree(db, repo_root, record) {
        Ok(git_commit_id) => {
            let updated = mark_committed(db, &record.deve_commit_id, &git_commit_id)?;
            report.committed = 1;
            report.records.push(updated);
        }
        Err(err) => {
            let updated = mark_out_of_sync(db, &record.deve_commit_id, err)?;
            report.out_of_sync = 1;
            report.records.push(updated);
        }
    }
    Ok(report)
}

fn mark_all_out_of_sync(
    db: &Database,
    records: Vec<GitMirrorRecord>,
    reason: String,
) -> Result<GitMirrorRunReport> {
    let mut report = GitMirrorRunReport {
        attempted: records.len(),
        ..GitMirrorRunReport::default()
    };
    for record in records {
        let updated = mark_out_of_sync(db, &record.deve_commit_id, reason.clone())?;
        report.out_of_sync += 1;
        report.records.push(updated);
    }
    Ok(report)
}

fn commit_worktree(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<String, String> {
    preflight_mirror_commit(db, repo_root, record)?;
    run_git(repo_root, &["add", "-A"])?;
    if !has_staged_changes(repo_root)? {
        if let Some(git_commit_id) = matching_head_commit(repo_root, record)? {
            return Ok(git_commit_id);
        }
        return Err("git mirror has no staged changes for queued Deve commit".to_string());
    }
    run_git(
        repo_root,
        &["commit", "--no-gpg-sign", "-m", &commit_message(record)],
    )?;
    Ok(run_git(repo_root, &["rev-parse", "HEAD"])?
        .trim()
        .to_string())
}

fn commit_message(record: &GitMirrorRecord) -> String {
    format!(
        "Mirror Deve commit {}\n\nDeve-Commit-Id: {}\nDeve-Ledger-Seq: {}\nDeve-Repo-Id: {}",
        record.deve_commit_id, record.deve_commit_id, record.ledger_seq, record.repo_id
    )
}

fn preflight_mirror_commit(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<(), String> {
    ensure_git_worktree(repo_root)?;
    ensure_notegit_is_not_tracked(repo_root)?;
    ensure_source_control_clean(db)?;
    ensure_git_changes_match_deve_commit(db, repo_root, record)?;
    Ok(())
}

fn ensure_git_worktree(repo_root: &Path) -> std::result::Result<(), String> {
    let inside = run_git(repo_root, &["rev-parse", "--is-inside-work-tree"])?;
    if inside.trim() == "true" {
        return Ok(());
    }
    Err(format!(
        "Git mirror is not a usable worktree: rev-parse returned {}",
        inside.trim()
    ))
}

fn ensure_notegit_is_not_tracked(repo_root: &Path) -> std::result::Result<(), String> {
    let paths = run_git_z_paths(repo_root, &["ls-files", "-z", "--", notegit::NOTE_GIT_DIR])?;
    if paths.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Git mirror refuses to run because {} is already tracked by Git",
        notegit::NOTE_GIT_DIR
    ))
}

fn ensure_source_control_clean(db: &Database) -> std::result::Result<(), String> {
    let pending = source_control::pending_fs::list_all(db)
        .map_err(|err| format!("failed to inspect pending source-control changes: {err}"))?;
    if !pending.is_empty() {
        return Err(format!(
            "Git mirror refuses to run with {} pending source-control change(s)",
            pending.len()
        ));
    }

    let staged = source_control::staging::list_staged_entries(db)
        .map_err(|err| format!("failed to inspect staged source-control changes: {err}"))?;
    if !staged.is_empty() {
        return Err(format!(
            "Git mirror refuses to run with {} staged source-control change(s)",
            staged.len()
        ));
    }
    Ok(())
}

fn ensure_git_changes_match_deve_commit(
    db: &Database,
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<(), String> {
    let expected = expected_mirror_paths(db, record)?;
    let changed = git_changed_paths(repo_root)?;
    let unexpected: Vec<_> = changed
        .into_iter()
        .filter(|path| notegit::is_internal_repo_path(path) || !expected.contains(path))
        .collect();
    if unexpected.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Git mirror refuses to include path(s) outside queued Deve commit: {}",
        unexpected.join(", ")
    ))
}

fn expected_mirror_paths(
    db: &Database,
    record: &GitMirrorRecord,
) -> std::result::Result<BTreeSet<String>, String> {
    let commit = load_deve_commit(db, &record.deve_commit_id)?;
    if commit.ledger_seq != record.ledger_seq {
        return Err(format!(
            "Git mirror record ledger_seq {} does not match Deve commit ledger_seq {}",
            record.ledger_seq, commit.ledger_seq
        ));
    }

    let diffs =
        source_control::commit_diff::compare_commits(db, commit.parent_id.as_deref(), &commit.id)
            .map_err(|err| format!("failed to compute queued Deve commit diff: {err}"))?;
    let mut paths = BTreeSet::new();
    paths.insert(".gitignore".to_string());
    for diff in diffs {
        paths.insert(to_forward_slash(&diff.path));
        if let Some(previous_path) = diff.previous_path {
            paths.insert(to_forward_slash(&previous_path));
        }
    }
    Ok(paths)
}

fn load_deve_commit(db: &Database, commit_id: &str) -> std::result::Result<CommitInfo, String> {
    let read_txn = db
        .begin_read()
        .map_err(|err| format!("failed to read Deve commit table: {err}"))?;
    let table = read_txn
        .open_table(source_control::commits::COMMITS_TABLE)
        .map_err(|err| format!("failed to open Deve commit table: {err}"))?;
    let raw = table
        .get(commit_id)
        .map_err(|err| format!("failed to load Deve commit {commit_id}: {err}"))?
        .ok_or_else(|| {
            format!("queued Git mirror record references missing Deve commit {commit_id}")
        })?;
    serde_json::from_str(raw.value())
        .map_err(|err| format!("failed to decode Deve commit {commit_id}: {err}"))
}

fn git_changed_paths(repo_root: &Path) -> std::result::Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    for args in [
        &["diff", "--name-only", "-z"][..],
        &["diff", "--cached", "--name-only", "-z"][..],
        &["ls-files", "-o", "--exclude-standard", "-z"][..],
    ] {
        paths.extend(run_git_z_paths(repo_root, args)?);
    }
    Ok(paths)
}

fn run_git(repo_root: &Path, args: &[&str]) -> std::result::Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(git_error(args, &output))
}

fn run_git_z_paths(repo_root: &Path, args: &[&str]) -> std::result::Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    if !output.status.success() {
        return Err(git_error(args, &output));
    }
    let mut paths = Vec::new();
    for raw in output.stdout.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let path = std::str::from_utf8(raw)
            .map_err(|err| format!("git {} returned non-UTF-8 path: {err}", args.join(" ")))?;
        paths.push(to_forward_slash(path));
    }
    Ok(paths)
}

fn has_staged_changes(repo_root: &Path) -> std::result::Result<bool, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .map_err(|err| format!("failed to run git diff --cached --quiet: {err}"))?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(git_error(&["diff", "--cached", "--quiet"], &output)),
    }
}

fn matching_head_commit(
    repo_root: &Path,
    record: &GitMirrorRecord,
) -> std::result::Result<Option<String>, String> {
    let body = match run_git(repo_root, &["log", "-1", "--pretty=%B"]) {
        Ok(body) => body,
        Err(_) => return Ok(None),
    };
    if !commit_body_matches_record(&body, record) {
        return Ok(None);
    }
    Ok(Some(
        run_git(repo_root, &["rev-parse", "HEAD"])?
            .trim()
            .to_string(),
    ))
}

fn commit_body_matches_record(body: &str, record: &GitMirrorRecord) -> bool {
    let expected_commit = format!("Deve-Commit-Id: {}", record.deve_commit_id);
    let expected_seq = format!("Deve-Ledger-Seq: {}", record.ledger_seq);
    let expected_repo = format!("Deve-Repo-Id: {}", record.repo_id);
    let has_commit = body.lines().any(|line| line.trim() == expected_commit);
    let has_seq = body.lines().any(|line| line.trim() == expected_seq);
    let has_repo = body.lines().any(|line| line.trim() == expected_repo);
    has_commit && has_seq && has_repo
}

fn git_error(args: &[&str], output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() { stderr } else { stdout };
    if detail.is_empty() {
        format!(
            "git {} failed with status {}",
            args.join(" "),
            output.status
        )
    } else {
        format!("git {} failed: {detail}", args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::{GitMirrorRunOptions, run_pending_mirror};
    use crate::git_bridge::{GitMirrorCommitState, get_record};
    use crate::ledger::RepoManager;
    use crate::source_control::pending_fs::{self, PendingFsEntry};
    use crate::source_control::{ChangeStatus, CommitInfo};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

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

    fn commit_deve_file(
        dir: &TempDir,
        repo: &RepoManager,
        path: &str,
        content: &str,
    ) -> CommitInfo {
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
    fn run_pending_mirror_commits_single_queued_record() {
        let (dir, repo, repo_root) = new_repo();
        let commit = commit_deve_file(&dir, &repo, "note.md", "hello\n");

        let report = run_for_default_repo(&repo, &repo_root);

        assert_eq!(report.attempted, 1);
        assert_eq!(report.committed, 1);
        let record = repo
            .run_on_local_repo(repo.local_repo_name(), |db| get_record(db, &commit.id))
            .expect("get")
            .expect("record");
        assert_eq!(record.state, GitMirrorCommitState::Committed);
        assert!(record.git_commit_id.is_some());
        let body = git(&repo_root, &["log", "-1", "--pretty=%B"]);
        assert!(body.contains(&format!("Deve-Commit-Id: {}", commit.id)));
        assert!(body.contains(&format!("Deve-Ledger-Seq: {}", commit.ledger_seq)));
    }
}

#[cfg(test)]
#[path = "executor_test.rs"]
mod executor_test;
