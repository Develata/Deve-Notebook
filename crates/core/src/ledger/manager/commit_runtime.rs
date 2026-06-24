//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!   - 03_storage/projection#projection-contract
//!
//! Source Control commit orchestration runtime.

use crate::config::GitBridgeMode;
#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::manager::git_mirror_queue_runtime;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::manager::{commit_plan, commit_preflight};
use crate::ledger::range;
use crate::models::DocId;
use crate::source_control::{ChangeEntry, CommitInfo, commits, ledger_dirty, staging};
use anyhow::Result;
use std::collections::HashSet;

pub(crate) struct CommitRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> CommitRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn commit_source_control_changes_with_git_bridge(
        &self,
        message: &str,
        git_bridge: GitBridgeMode,
    ) -> Result<CommitInfo> {
        self.commit_source_control_changes_in_local_repo_with_git_bridge(
            self.manager.local_repo_name(),
            message,
            git_bridge,
        )
    }

    pub(crate) fn commit_source_control_changes_in_local_repo_with_git_bridge(
        &self,
        repo_name: &str,
        message: &str,
        git_bridge: GitBridgeMode,
    ) -> Result<CommitInfo> {
        let message = message.trim();
        if message.is_empty() {
            anyhow::bail!("source control commit requires a non-empty message");
        }
        let staged = self
            .manager
            .run_on_local_repo(repo_name, staging::list_staged_entries)?;
        let confirmed = self
            .manager
            .run_on_local_repo(repo_name, ledger_dirty::list_confirmed)?;
        let mut targets = commit_plan::build_targets(staged);
        if targets.is_empty() && confirmed.is_empty() {
            anyhow::bail!("Nothing to commit: staging and confirmed ledger changes are empty");
        }
        targets.sort_by_key(|target| target.delete_only);
        if !targets.is_empty() {
            commit_preflight::preflight_staged_commit_targets(self.manager, repo_name, &targets)?;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let git_mirror_repo_id = if git_bridge == GitBridgeMode::Mirror {
            git_mirror_queue_runtime::queue_repo_id(self.manager, repo_name)
        } else {
            None
        };

        let doc_count = covered_doc_count(&targets, &confirmed);
        for target in &targets {
            if target.delete_only {
                self.manager.commit_delete_snapshot_in_local_repo(
                    repo_name,
                    &target.path,
                    target.doc_id,
                )?;
            } else {
                self.manager.commit_file_ops_in_local_repo(
                    repo_name,
                    &target.path,
                    target.doc_id,
                )?;
            }
        }

        let commit = self.manager.run_on_local_repo(repo_name, |db| {
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
            staging::clear(db)?;
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
}

fn covered_doc_count(targets: &[commit_plan::CommitTarget], confirmed: &[ChangeEntry]) -> u32 {
    let mut keys = HashSet::new();
    for target in targets {
        insert_covered_key(&mut keys, target.doc_id, &target.path);
    }
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

impl RepoManager {
    pub(crate) fn commit_runtime(&self) -> CommitRuntime<'_> {
        CommitRuntime::new(self)
    }
}
