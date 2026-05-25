//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/index#internal-path-normalization
//!
use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use anyhow::Result;

impl RepoManager {
    pub fn stage_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.source_control_runtime()
            .stage_pending_target_in_local_repo(repo_name, target)
    }

    pub fn stage_resolved_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.source_control_runtime()
            .stage_resolved_pending_target_in_local_repo(repo_name, target)
    }

    pub fn stage_resolved_pending_targets_in_local_repo(
        &self,
        repo_name: &str,
        targets: &[ScPathTarget],
    ) -> Result<()> {
        self.source_control_runtime()
            .stage_resolved_pending_targets_in_local_repo(repo_name, targets)
    }

    pub fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.source_control_runtime()
            .discard_pending_target_in_local_repo(repo_name, target)
    }

    pub fn unstage_file_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        self.source_control_runtime()
            .unstage_file_target_in_local_repo(repo_name, target)
    }
}
