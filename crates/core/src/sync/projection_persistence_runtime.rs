//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/projection#durable-projection-fault-contract
//!   - 04_repository#tree-projection-contract

use super::{PreparedLocalRepoMaterialization, SyncManager, materialize};
use anyhow::Result;

impl SyncManager {
    /// Pre-condition: `repo_name` 必须已解析为真实本地 repo 名称。
    pub fn materialize_local_repo(&self, repo_name: &str) -> Result<()> {
        match materialize::materialize_local_repo(&self.repo, &self.persist_guard, repo_name) {
            Ok(()) => Ok(()),
            Err(err) => {
                if materialize::is_broken_structure_projection_error(&err) {
                    if let Err(fault_error) = self.mark_projection_rebuild_fault(repo_name, &err) {
                        return Err(err.context(format!(
                            "failed to persist Projection Fault evidence: {fault_error}"
                        )));
                    }
                } else if let Err(fault_error) =
                    self.mark_projection_writeback_fault_for_path(repo_name, "", &err)
                {
                    return Err(err.context(format!(
                        "failed to persist Projection Fault evidence: {fault_error}"
                    )));
                }
                Err(err)
            }
        }
    }

    /// Builds the projection write plan without holding the server repo permit.
    pub fn prepare_local_repo_materialization(
        &self,
        repo_name: &str,
    ) -> Result<PreparedLocalRepoMaterialization> {
        match materialize::prepare_local_repo_materialization(&self.repo, repo_name) {
            Ok(prepared) => Ok(prepared),
            Err(error) => {
                if let Err(fault_error) = self.record_materialization_error(repo_name, &error) {
                    return Err(error.context(format!(
                        "failed to persist Projection Fault evidence: {fault_error}"
                    )));
                }
                Err(error)
            }
        }
    }

    /// Applies a previously prepared plan after the server gate revalidates
    /// repo identity, mount generation and the mutation lane.
    pub fn apply_prepared_local_repo_materialization(
        &self,
        repo_name: &str,
        prepared: PreparedLocalRepoMaterialization,
    ) -> Result<()> {
        match materialize::apply_prepared_local_repo_materialization(
            &self.repo,
            &self.persist_guard,
            prepared,
        ) {
            Ok(()) => Ok(()),
            Err(error) => {
                if let Err(fault_error) = self.record_materialization_error(repo_name, &error) {
                    return Err(error.context(format!(
                        "failed to persist Projection Fault evidence: {fault_error}"
                    )));
                }
                Err(error)
            }
        }
    }

    fn record_materialization_error(&self, repo_name: &str, error: &anyhow::Error) -> Result<()> {
        if materialize::is_broken_structure_projection_error(error) {
            self.mark_projection_rebuild_fault(repo_name, error)
        } else {
            self.mark_projection_writeback_fault_for_path(repo_name, "", error)
        }
    }
}
