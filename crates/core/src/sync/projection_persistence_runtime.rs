//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 04_repository#tree-projection-contract

use super::{SyncManager, materialize, projection_fault_journal};
use anyhow::Result;

impl SyncManager {
    /// Pre-condition: `repo_name` 必须已解析为真实本地 repo 名称。
    pub fn materialize_local_repo(&self, repo_name: &str) -> Result<()> {
        match materialize::materialize_local_repo(&self.repo, &self.persist_guard, repo_name) {
            Ok(()) => {
                projection_fault_journal::clear_faults_for_repo(&self.repo, repo_name)?;
                self.clear_projection_degraded(repo_name);
                Ok(())
            }
            Err(err) => {
                if materialize::is_broken_structure_projection_error(&err) {
                    self.mark_projection_degraded(repo_name);
                } else {
                    self.mark_projection_writeback_fault_for_path(repo_name, "", &err);
                }
                Err(err)
            }
        }
    }
}
