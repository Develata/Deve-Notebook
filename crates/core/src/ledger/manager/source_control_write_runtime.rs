//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!
//! Source Control write/commit runtime.

use crate::config::GitBridgeMode;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::source_control;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeStatus, CommitInfo, pending_fs, staging};
use anyhow::Result;
use std::collections::HashSet;

pub(crate) struct SourceControlWriteRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> SourceControlWriteRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn unstage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self
            .manager
            .tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.unstage_file_target_in_local_repo(repo_name, &target)
    }

    pub(crate) fn commit_staged_in_local_repo_with_git_bridge(
        &self,
        repo_name: &str,
        message: &str,
        git_bridge: GitBridgeMode,
    ) -> Result<CommitInfo> {
        if repo_name == self.manager.local_repo_name() {
            return self
                .manager
                .commit_runtime()
                .commit_staged_with_ops_with_git_bridge(message, git_bridge);
        }
        self.manager
            .commit_runtime()
            .commit_staged_with_ops_in_local_repo_with_git_bridge(repo_name, message, git_bridge)
    }

    pub(crate) fn stage_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self
            .manager
            .tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.stage_pending_target_in_local_repo(repo_name, &target)
    }

    pub(crate) fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self
            .manager
            .tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.discard_pending_target_in_local_repo(repo_name, &target)
    }

    pub(crate) fn stage_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.manager.run_on_local_repo(repo_name, |db| {
            let Some(entry) = pending_fs::get_for_target(db, target)? else {
                if staging::get_staged_for_target(db, target)?.is_some() {
                    return Ok(());
                }
                anyhow::bail!("Path is not in pending_fs_ops: {}", target.path);
            };
            let entries = collect_stage_entries_for_pending_target(db, entry)?;
            for entry in &entries {
                ensure_pending_entry_stageable(entry)?;
            }
            for entry in entries {
                pending_fs::remove(db, &entry.path)?;
                source_control::stage_pending_entry(db, &entry)?;
            }
            Ok(())
        })
    }

    pub(crate) fn stage_resolved_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.stage_resolved_pending_targets_in_local_repo(repo_name, std::slice::from_ref(target))
    }

    pub(crate) fn stage_resolved_pending_targets_in_local_repo(
        &self,
        repo_name: &str,
        targets: &[ScPathTarget],
    ) -> Result<()> {
        self.manager.run_on_local_repo(repo_name, |db| {
            let mut selected = Vec::new();
            let mut seen_paths = HashSet::new();
            for target in targets {
                let Some(entry) = pending_fs::get_for_target(db, target)? else {
                    if staging::get_staged_for_target(db, target)?.is_some() {
                        continue;
                    }
                    anyhow::bail!("Path is not in pending_fs_ops: {}", target.path);
                };
                for entry in collect_stage_entries_for_pending_target(db, entry)? {
                    if seen_paths.insert(entry.path.clone()) {
                        selected.push(entry);
                    }
                }
            }
            for mut entry in selected {
                pending_fs::remove(db, &entry.path)?;
                entry.has_conflict = false;
                source_control::stage_pending_entry(db, &entry)?;
            }
            Ok(())
        })
    }

    pub(crate) fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path = self
            .manager
            .run_on_local_repo(repo_name, |db| pending_fs::get_for_target(db, target))?
            .map(|entry| entry.path)
            .ok_or_else(|| anyhow::anyhow!("Path is not in pending_fs_ops: {}", target.path))?;
        self.manager
            .discard_pending_workdir_in_local_repo(repo_name, &path)
    }

    pub(crate) fn unstage_file_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.manager.run_on_local_repo(repo_name, |db| {
            let Some((path, staged)) = staging::take_staged_for_target(db, target)? else {
                anyhow::bail!("Path is not staged: {}", target.path);
            };
            pending_fs::upsert(
                db,
                &pending_fs::PendingFsEntry {
                    path,
                    renamed_from: staged.renamed_from,
                    doc_id: staged.doc_id,
                    change_type: staged.status,
                    content_hash: staged.content_hash,
                    detected_at: chrono::Utc::now().timestamp_millis(),
                    has_conflict: staged.has_conflict,
                },
            )
        })
    }
}

fn ensure_pending_entry_stageable(entry: &pending_fs::PendingFsEntry) -> Result<()> {
    if entry.has_conflict {
        anyhow::bail!("unresolved source control conflict: {}", entry.path);
    }
    Ok(())
}

fn collect_stage_entries_for_pending_target(
    db: &redb::Database,
    entry: pending_fs::PendingFsEntry,
) -> Result<Vec<pending_fs::PendingFsEntry>> {
    let mut entries = vec![entry.clone()];
    let Some(renamed_from) = entry.renamed_from.as_deref() else {
        return Ok(entries);
    };

    for candidate in pending_fs::list_all(db)? {
        if candidate.path == renamed_from
            && candidate.doc_id == entry.doc_id
            && candidate.change_type == ChangeStatus::Deleted
            && !entries
                .iter()
                .any(|existing| existing.path == candidate.path)
        {
            entries.push(candidate);
        }
    }
    Ok(entries)
}
