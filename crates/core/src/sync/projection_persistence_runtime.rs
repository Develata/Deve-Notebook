//! plan_ref:
//!   - 04_storage#projection-contract
//!   - 06_repository#tree-projection-contract

use super::{SyncManager, materialize};
use anyhow::Result;

impl SyncManager {
    /// Pre-condition: `repo_name` 必须已解析为真实本地 repo 名称。
    pub fn materialize_local_repo(&self, repo_name: &str) -> Result<()> {
        match materialize::materialize_local_repo(&self.repo, &self.persist_guard, repo_name) {
            Ok(()) => {
                self.clear_projection_degraded(repo_name);
                Ok(())
            }
            Err(err) => {
                if materialize::is_broken_structure_projection_error(&err) {
                    self.mark_projection_degraded(repo_name);
                }
                Err(err)
            }
        }
    }
}
