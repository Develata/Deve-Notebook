//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 04_storage#facts-partition
//!   - 03_rendering#document-authority-bridge
//!   - 04_storage#projection-contract
//!
//! Source Control commit orchestration runtime.

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::manager::git_mirror_queue_runtime;
use crate::ledger::manager::types::RepoManager;
use crate::ledger::manager::{commit_plan, commit_preflight};
use crate::ledger::range;
use crate::source_control::{CommitInfo, commits, staging};
use anyhow::Result;
use std::path::PathBuf;

pub(crate) struct CommitRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> CommitRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn commit_staged_with_ops(
        &self,
        message: &str,
        vault_root: PathBuf,
    ) -> Result<CommitInfo> {
        self.commit_staged_with_ops_in_local_repo(
            self.manager.local_repo_name(),
            message,
            vault_root,
        )
    }

    pub(crate) fn commit_staged_with_ops_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
        vault_root: PathBuf,
    ) -> Result<CommitInfo> {
        let staged = self
            .manager
            .run_on_local_repo(repo_name, staging::list_staged_entries)?;
        if staged.is_empty() {
            anyhow::bail!("Nothing to commit: staging area is empty");
        }
        let mut targets = commit_plan::build_targets(staged);
        targets.sort_by_key(|target| target.delete_only);
        commit_preflight::preflight_staged_commit_targets(self.manager, repo_name, &targets)?;
        #[cfg(not(target_arch = "wasm32"))]
        let git_mirror_repo_id = git_mirror_queue_runtime::queue_repo_id(self.manager, repo_name);

        let doc_count = targets.len() as u32;
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
                    &vault_root,
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

impl RepoManager {
    pub(crate) fn commit_runtime(&self) -> CommitRuntime<'_> {
        CommitRuntime::new(self)
    }
}
