//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/projection#projection-contract
//!
//! Source Control commit orchestration runtime.

use crate::ledger::manager::commit_plan::CommitTarget;
#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::manager::git_mirror_queue_runtime;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::manager::{commit_plan, commit_preflight};
use crate::ledger::{ops, range};
use crate::models::DocId;
use crate::source_control::{
    ChangeEntry, ChangeStatus, CommitInfo, changes, commits, external_overlap, ledger_dirty,
    staging,
};
use anyhow::Result;
use redb::Database;
use std::collections::HashSet;

pub(crate) struct CommitRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> CommitRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn commit_source_control_changes(&self, message: &str) -> Result<CommitInfo> {
        self.commit_source_control_changes_in_local_repo(self.manager.local_repo_name(), message)
    }

    pub(crate) fn commit_source_control_changes_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> Result<CommitInfo> {
        let message = message.trim();
        if message.is_empty() {
            anyhow::bail!("source control commit requires a non-empty message");
        }
        let confirmed = self
            .manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
        if confirmed.is_empty() {
            anyhow::bail!("Nothing to commit: confirmed ledger changes are empty");
        }
        #[cfg(not(target_arch = "wasm32"))]
        let git_mirror_repo_id = git_mirror_queue_runtime::queue_repo_id(self.manager, repo_name);

        let final_confirmed = self
            .manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
        let doc_count = covered_doc_count(&final_confirmed);

        let commit = self.manager.run_on_local_repo(repo_name, |db| {
            sync_confirmed_commit_snapshots(db, &final_confirmed)?;
            let ledger_seq = range::get_max_seq(db)?;
            let commit = commits::create(db, message, doc_count, ledger_seq)?;
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(repo_id) = git_mirror_repo_id
                && let Err(err) = crate::git_bridge::queue_deve_commit(db, repo_id, &commit)
            {
                tracing::warn!(
                    repo_name,
                    deve_commit_id = %commit.id,
                    error = %err,
                    "Git mirror queue update failed after Deve commit; ledger commit is kept"
                );
            }
            Ok(commit)
        })?;
        tracing::info!(
            "Committed {} files in {}: {}",
            doc_count,
            repo_name,
            message
        );
        Ok(commit)
    }

    pub(crate) fn apply_external_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        let staged = self
            .manager
            .run_on_local_repo(repo_name, staging::list_staged_entries)?;
        let confirmed = self
            .manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
        let mut targets = commit_plan::build_targets(staged);
        if targets.is_empty() {
            anyhow::bail!("No external changes staged to apply");
        }
        targets.sort_by_key(|target| target.delete_only);
        ensure_external_targets_do_not_overlap_confirmed(&targets, &confirmed)?;
        commit_preflight::preflight_staged_commit_targets(self.manager, repo_name, &targets)?;

        for target in &targets {
            if target.delete_only {
                self.manager.apply_external_delete_in_local_repo(
                    repo_name,
                    &target.path,
                    target.doc_id,
                )?;
            } else {
                self.manager.apply_external_file_ops_in_local_repo(
                    repo_name,
                    &target.path,
                    target.doc_id,
                )?;
            }
        }

        let final_confirmed = self
            .manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
        self.manager.run_on_local_repo(repo_name, staging::clear)?;
        tracing::info!(
            "Applied {} external changes to ledger in {}",
            targets.len(),
            repo_name
        );
        Ok(final_confirmed)
    }
}

fn ensure_external_targets_do_not_overlap_confirmed(
    targets: &[CommitTarget],
    confirmed: &[ChangeEntry],
) -> Result<()> {
    for target in targets {
        if external_overlap::fields_overlap_any_confirmed(
            target.doc_id,
            &target.path,
            target.renamed_from.as_deref(),
            confirmed,
        ) {
            anyhow::bail!(
                "external change overlaps confirmed ledger changes: {}",
                target.path
            );
        }
    }
    Ok(())
}

fn covered_doc_count(confirmed: &[ChangeEntry]) -> u32 {
    let mut keys = HashSet::new();
    for entry in confirmed {
        insert_covered_key(&mut keys, entry.doc_id, &entry.path);
    }
    keys.len() as u32
}

fn insert_covered_key(keys: &mut HashSet<String>, doc_id: Option<DocId>, path: &str) {
    let key = match doc_id {
        Some(doc_id) => format!("doc:{doc_id}"),
        None => format!("path:{path}"),
    };
    keys.insert(key);
}

fn sync_confirmed_commit_snapshots(db: &Database, confirmed: &[ChangeEntry]) -> Result<()> {
    for entry in confirmed {
        let Some(doc_id) = entry.doc_id else {
            continue;
        };
        if entry.status == ChangeStatus::Deleted {
            changes::remove_snapshot(db, doc_id)?;
            continue;
        }
        let entries = ops::get_ops_from_db(db, doc_id)?;
        let facts: Vec<_> = entries.into_iter().map(|(_, entry)| entry).collect();
        let content = crate::state::reconstruct_content(&facts);
        changes::save_snapshot(db, doc_id, &content)?;
    }
    Ok(())
}

impl RepoManager {
    pub(crate) fn commit_runtime(&self) -> CommitRuntime<'_> {
        CommitRuntime::new(self)
    }
}
