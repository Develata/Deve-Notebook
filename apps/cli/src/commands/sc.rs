//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 14_commands#cli-commands
//!
//! Minimal Deve Source Control CLI surface.

use crate::commands::repo_arg::resolve_local_repo_args;
use crate::commands::source_control_workspace_gate::ensure_local_repo_workspace_identity_for_write;
use anyhow::{Result, bail};
use clap::Subcommand;
use deve_core::config::GitBridgeMode;
use deve_core::ledger::RepoManager;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;
use std::path::Path;

#[derive(Subcommand, Debug)]
pub(crate) enum ScAction {
    /// Print staged and unstaged source-control counts
    Status {
        #[arg(long)]
        repo: Option<String>,
    },
    /// Stage pending source-control changes
    Stage {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Commit staged source-control changes
    Commit {
        #[arg(long)]
        repo: Option<String>,
        #[arg(short, long)]
        message: String,
    },
}

pub fn status(ledger_dir: &Path, target_repo: Option<&str>, snapshot_depth: usize) -> Result<()> {
    crate::commands::sc_status::run(ledger_dir, target_repo, snapshot_depth)
}

pub fn stage(
    ledger_dir: &Path,
    target_repo: Option<&str>,
    all: bool,
    snapshot_depth: usize,
) -> Result<()> {
    require_stage_all(all)?;
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
        ensure_no_unresolved_conflicts(&pending)?;
        let targets = targets_from_entries(&pending);
        ensure_local_repo_workspace_identity_for_write(&repo, &repo_name, "source-control write")?;
        repo.stage_resolved_pending_targets_in_local_repo(&repo_name, &targets)?;
        println!("sc_stage[{repo_name}]: staged={}", targets.len());
    }
    Ok(())
}

pub fn commit(
    ledger_dir: &Path,
    target_repo: Option<&str>,
    message: &str,
    snapshot_depth: usize,
    git_bridge: GitBridgeMode,
) -> Result<()> {
    let message = message.trim();
    if message.is_empty() {
        bail!("sc commit requires a non-empty --message");
    }
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, target_repo)?;
    for repo_name in repo_names {
        ensure_local_repo_workspace_identity_for_write(&repo, &repo_name, "source-control write")?;
        let commit =
            repo.commit_staged_in_local_repo_with_git_bridge(&repo_name, message, git_bridge)?;
        println!(
            "sc_commit[{repo_name}]: id={} ledger_seq={} files={}",
            commit.id, commit.ledger_seq, commit.doc_count
        );
    }
    Ok(())
}

fn require_stage_all(all: bool) -> Result<()> {
    if !all {
        bail!("sc stage currently requires --all");
    }
    Ok(())
}

fn ensure_no_unresolved_conflicts(entries: &[ChangeEntry]) -> Result<()> {
    if let Some(entry) = entries.iter().find(|entry| entry.has_conflict) {
        bail!("unresolved source control conflict: {}", entry.path);
    }
    Ok(())
}

fn targets_from_entries(entries: &[ChangeEntry]) -> Vec<ScPathTarget> {
    entries
        .iter()
        .map(|entry| ScPathTarget {
            path: entry.path.clone(),
            doc_id: entry.doc_id,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{commit, require_stage_all, stage, targets_from_entries};
    use deve_core::config::GitBridgeMode;
    use deve_core::ledger::RepoManager;
    use deve_core::models::DocId;
    use deve_core::source_control::pending_fs::{self, PendingFsEntry};
    use deve_core::source_control::staging;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};
    use uuid::Uuid;

    #[test]
    fn stage_requires_explicit_all() {
        assert!(require_stage_all(false).is_err());
        assert!(require_stage_all(true).is_ok());
    }

    #[test]
    fn targets_preserve_doc_identity() {
        let doc_id = DocId::from_u128(7);
        let targets = targets_from_entries(&[ChangeEntry {
            path: "gone.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            status: ChangeStatus::Deleted,
            has_conflict: false,
        }]);

        assert_eq!(targets[0].path, "gone.md");
        assert_eq!(targets[0].doc_id, Some(doc_id));
    }

    #[test]
    fn sc_stage_all_rejects_unresolved_conflict() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger_dir = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let doc_id = {
            let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
            repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
            let (doc_id, _ops) =
                repo.apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")?;
            repo.run_on_local_repo("default", |db| {
                pending_fs::upsert(
                    db,
                    &PendingFsEntry {
                        path: "notes/a.md".into(),
                        renamed_from: None,
                        doc_id: Some(doc_id),
                        change_type: ChangeStatus::Modified,
                        content_hash: pending_fs::content_hash("dirty"),
                        detected_at: 1,
                        has_conflict: true,
                    },
                )
            })?;
            doc_id
        };

        let err = stage(&ledger_dir, Some("default"), true, 10)
            .expect_err("stage --all must reject unresolved conflicts");

        assert!(
            err.to_string()
                .contains("unresolved source control conflict"),
            "unexpected error: {}",
            err
        );
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        let pending = repo.run_on_local_repo("default", pending_fs::list_all)?;
        let staged = repo.run_on_local_repo("default", staging::list_staged_entries)?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].doc_id, Some(doc_id));
        assert!(pending[0].has_conflict);
        assert!(staged.is_empty());
        Ok(())
    }

    #[test]
    fn sc_stage_all_rejects_broken_workspace_identity() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger_dir = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let workspace = {
            let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
            repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
            let workspace = repo.ensure_local_repo_workspace_identity("default")?;
            repo.run_on_local_repo("default", |db| {
                pending_fs::upsert(
                    db,
                    &PendingFsEntry {
                        path: "notes/a.md".into(),
                        renamed_from: None,
                        doc_id: None,
                        change_type: ChangeStatus::Added,
                        content_hash: pending_fs::content_hash("dirty"),
                        detected_at: 1,
                        has_conflict: false,
                    },
                )
            })?;
            workspace
        };
        corrupt_workspace_identity(&workspace)?;

        let err = stage(&ledger_dir, Some("default"), true, 10)
            .expect_err("stage --all must reject a broken workspace identity");

        assert_identity_gate_error(&err);
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        let pending = repo.run_on_local_repo("default", pending_fs::list_all)?;
        let staged = repo.run_on_local_repo("default", staging::list_staged_entries)?;
        assert_eq!(pending.len(), 1);
        assert!(staged.is_empty());
        Ok(())
    }

    #[test]
    fn sc_commit_rejects_broken_workspace_identity() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger_dir = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let workspace = {
            let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
            repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
            let workspace = repo.ensure_local_repo_workspace_identity("default")?;
            let file = repo.local_repo_workspace_path("default", "notes/a.md")?;
            std::fs::create_dir_all(file.parent().expect("workspace file must have a parent"))?;
            std::fs::write(&file, "dirty")?;
            repo.run_on_local_repo("default", |db| {
                pending_fs::upsert(
                    db,
                    &PendingFsEntry {
                        path: "notes/a.md".into(),
                        renamed_from: None,
                        doc_id: None,
                        change_type: ChangeStatus::Added,
                        content_hash: pending_fs::content_hash("dirty"),
                        detected_at: 1,
                        has_conflict: false,
                    },
                )
            })?;
            repo.stage_pending_in_local_repo("default", "notes/a.md")?;
            workspace
        };
        corrupt_workspace_identity(&workspace)?;

        let err = commit(
            &ledger_dir,
            Some("default"),
            "commit broken identity",
            10,
            GitBridgeMode::Mirror,
        )
        .expect_err("commit must reject a broken workspace identity");

        assert_identity_gate_error(&err);
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        let staged = repo.run_on_local_repo("default", staging::list_staged_entries)?;
        let commits = repo.list_commits_in_local_repo("default", 10)?;
        assert_eq!(staged.len(), 1);
        assert!(commits.is_empty());
        Ok(())
    }

    fn corrupt_workspace_identity(workspace: &std::path::Path) -> anyhow::Result<()> {
        std::fs::write(
            deve_core::utils::notegit::repo_identity_path(workspace),
            format!(
                "version = 1\nrepo_id = \"{}\"\nrepo_name = \"default\"\n",
                Uuid::new_v4()
            ),
        )?;
        Ok(())
    }

    fn assert_identity_gate_error(err: &anyhow::Error) {
        let message = err.to_string();
        assert!(
            message.contains("identity marker") || message.contains("workspace identity"),
            "unexpected error: {}",
            message
        );
    }
}
